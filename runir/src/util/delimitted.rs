use std::{
    borrow::Cow,
    convert::Infallible,
    marker::PhantomData,
    ops::{Range, RangeTo},
    str::FromStr,
    sync::atomic::AtomicUsize,
};

/// Type-alias for a Delimitted<',', String>,
pub type CommaSeperatedStrings = Delimitted<',', String>;

/// Type-alias for a Delimitted<'.', String>,
pub type DotSeperatedStrings = Delimitted<'.', String>;

/// Type-alias for a key value pair pattern
pub type KeyValuePairs = Delimitted<';', Delimitted<'=', String>>;

/// Type-alias for a key value pair pattern, where the value is a vec of values
pub type KeyValueVecPairs<T> = Delimitted<';', Delimitted<'=', Delimitted<',', T>>>;

/// Struct to define an iterater over a list of strings w/ a character DELIM
#[derive(PartialEq, PartialOrd)]
pub struct Delimitted<const DELIM: char, T: FromStr + Send + Sync + 'static> {
    /// Source of the list
    value: String,
    /// Current iterator position
    cursor: usize,
    _t: PhantomData<T>,
}

/// Struct to define an iterater over a list of strings w/ a character DELIM
#[derive(Debug)]
pub struct DelimittedStr<const DELIM: char> {
    /// Source of the list
    value: String,
    /// Current iterator position
    cursor: AtomicUsize,
}

impl KeyValuePairs {
    /// Returns an iterator of key value pairs
    pub fn into_pairs(self) -> impl Iterator<Item = (String, String)> {
        self.map(|mut h| (h.next(), h.next()))
            .filter_map(|(k, v)| match (k, v) {
                (Some(k), Some(v)) => Some((k, v)),
                _ => None,
            })
    }
}

/// Scans for headers from a string and returns an iterator over the results
#[inline]
pub fn scan_for_headers(source: &str) -> impl Iterator<Item = (&str, Vec<&str>)> {
    let headers = source.parse::<DelimittedStr<';'>>().unwrap();

    let items = headers
        .map(|h| {
            let pair = DelimittedStr::<'='>::from_str(&h).unwrap();
            let key = (&pair).next().unwrap();
            let mut values = vec![];
            for v in &pair {
                let list = DelimittedStr::<','>::from_str(&v).unwrap();
                for k in &list {
                    values.push(Item(k.len()));
                }
                if values.is_empty() {
                    values.push(Item(v.len()));
                }
            }
            (Item(key.len()), values)
        })
        .collect::<Vec<_>>();

    StrIter::new(source, items, |kvp, cursor, source| {
        let (k, v) = kvp;
        let key = k.span(*cursor).view(source);
        *cursor += k.0 + 1;

        let mut vals = vec![];
        for _v in v {
            let val = _v.span(*cursor).view(source);
            *cursor += _v.0 + 1;
            vals.push(val);
        }

        (key, vals)
    })
}

#[derive(Clone, Copy, Debug)]
struct Item(usize);

impl Item {
    fn span(&self, cursor: usize) -> Span {
        let Item(len) = self;
        if cursor == 0 {
            Span::Start(..*len)
        } else {
            Span::Inner(cursor..cursor + len)
        }
    }
}

#[derive(Debug)]
enum Span {
    Start(RangeTo<usize>),
    Inner(Range<usize>),
}

impl Span {
    fn view<'a>(&self, buffer: &'a str) -> &'a str {
        match self {
            Span::Start(s) => &buffer[*s],
            Span::Inner(i) => &buffer[i.start..i.end],
        }
    }
}

struct StrIter<'a, F, T> {
    s: &'a str,
    cursor: usize,
    items: Vec<F>,
    item_fn: fn(F, &mut usize, &'a str) -> T,
}

impl<'a, F, T> StrIter<'a, F, T> {
    pub fn new(
        source: &'a str,
        mut items: Vec<F>,
        item_fn: fn(F, &mut usize, &'a str) -> T,
    ) -> Self {
        items.reverse();
        StrIter {
            s: source,
            cursor: 0,
            items,
            item_fn,
        }
    }
}

impl<'a, F, T> Iterator for StrIter<'a, F, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.items
            .pop()
            .map(|i| (self.item_fn)(i, &mut self.cursor, self.s))
    }
}

impl<T: FromStr + Default + Send + Sync + 'static> KeyValueVecPairs<T> {
    /// Returns an iterator of key value pairs
    pub fn into_pairs(self) -> impl Iterator<Item = (T, Vec<T>)> {
        self.map(|mut h| (h.next(), h.next()))
            .filter_map(|(k, v)| match (k, v) {
                (Some(mut k), Some(v)) => {
                    Some((k.next().unwrap_or_default(), v.collect::<Vec<_>>()))
                }
                _ => None,
            })
    }
}

impl<const DELIM: char, T: FromStr + Send + Sync + 'static> Clone for Delimitted<DELIM, T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            cursor: self.cursor,
            _t: PhantomData,
        }
    }
}

impl<const DELIM: char, T: FromStr + Send + Sync + 'static> std::fmt::Debug
    for Delimitted<DELIM, T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Delimitted")
            .field("value", &self.value)
            .field("cursor", &self.cursor)
            .finish()
    }
}

impl<const DELIM: char, T: FromStr + Send + Sync + 'static> FromStr for Delimitted<DELIM, T> {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Delimitted {
            value: s.to_string(),
            cursor: 0,
            _t: PhantomData,
        })
    }
}

impl<const DELIM: char, T: FromStr + Send + Sync + 'static> Iterator for Delimitted<DELIM, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let mut value = self.value.split(DELIM).skip(self.cursor);
        self.cursor += 1;
        value.next().and_then(|v| T::from_str(v.trim()).ok())
    }
}

impl<const DELIM: char> FromStr for DelimittedStr<DELIM> {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(DelimittedStr {
            value: s.replace(' ', ""),
            cursor: AtomicUsize::new(0),
        })
    }
}

impl<const DELIM: char> DelimittedStr<DELIM> {
    /// Returns the next value
    fn increment_counter(&self) {
        self.cursor
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Returns the current value
    fn value<'a: 'b, 'b>(&'a self) -> std::option::Option<Cow<'b, str>> {
        self.value
            .split(DELIM)
            .nth(self.cursor.load(std::sync::atomic::Ordering::Relaxed))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(Cow::Borrowed)
    }
}

impl<'a, const DELIM: char> Iterator for &'a DelimittedStr<DELIM> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.value.contains(DELIM) {
            return None;
        }
        let out = self.value();
        self.increment_counter();

        if out.is_none() {
            self.cursor.store(0, std::sync::atomic::Ordering::Relaxed)
        }
        out
    }
}

impl<const DELIM: char, T: FromStr + Send + Sync + 'static> Default for Delimitted<DELIM, T> {
    fn default() -> Self {
        Self::from_str("").expect("should be able to parse an empty str")
    }
}

#[test]
fn test_delimitted_complex() {
    let mut headers = "Accept = text; Names = 'test', 'abc';"
        .parse::<Delimitted<';', String>>()
        .unwrap();

    assert_eq!("Accept = text", headers.next().unwrap());
    assert_eq!("Names = 'test', 'abc'", headers.next().unwrap());

    let mut headers = "Accept = text; Names = 'test', 'abc';"
        .parse::<Delimitted<';', Delimitted<'=', String>>>()
        .unwrap()
        .into_pairs();

    assert_eq!(
        ("Accept".to_string(), "text".to_string()),
        headers.next().unwrap()
    );
    assert_eq!(
        ("Names".to_string(), "'test', 'abc'".to_string()),
        headers.next().unwrap()
    );

    let headers = "Accept = text; Names = 'test', 'abc';"
        .parse::<Delimitted<';', Delimitted<'=', Delimitted<',', String>>>>()
        .unwrap()
        .into_pairs();

    for h in headers {
        println!("{h:?}");
    }
    ()
}

#[test]
fn test_delimitted_str() {
    let mut results = scan_for_headers("Accept=text;Names=test,abc;");
    assert_eq!(("Accept", vec!["text"]), results.next().unwrap());
    assert_eq!(("Names", vec!["test", "abc"]), results.next().unwrap());
}
