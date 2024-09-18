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
pub struct Delimitted<const DELIM: char, T: FromStr + Send + Sync + 'static, const COUNT: usize = 0> {
    /// Source of the list
    value: String,
    /// Current iterator position
    cursor: usize,
    _t: PhantomData<T>,
}

/// Struct to define an iterater over a list of strings w/ a character DELIM
#[derive(Debug)]
pub struct DelimittedStr<const DELIM: char, const COUNT: usize = 0> {
    /// Source of the list
    value: String,
    /// Current iterator position
    cursor: AtomicUsize,
}

/// Struct to define an iterater over a list of strings w/ a character DELIM1DELIM2
#[derive(Debug)]
pub struct DelimittedStr2<const DELIM1: char, const DELIM2: char, const COUNT: usize = 0> {
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
    let headers = source.parse::<DelimittedStr2<';', ';'>>().unwrap();

    let items = headers
        .map(|h| {
            let pair = DelimittedStr::<'=', 2>::from_str(&h).unwrap();
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

        *cursor += 1;
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

impl<const DELIM: char, T: FromStr + Send + Sync + 'static, const COUNT: usize> Clone for Delimitted<DELIM, T, COUNT> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            cursor: self.cursor,
            _t: PhantomData,
        }
    }
}

impl<const DELIM: char, T: FromStr + Send + Sync + 'static, const COUNT: usize> std::fmt::Debug
    for Delimitted<DELIM, T, COUNT>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Delimitted")
            .field("value", &self.value)
            .field("cursor", &self.cursor)
            .finish()
    }
}

impl<const DELIM: char, const COUNT: usize, T: FromStr + Send + Sync + 'static> FromStr for Delimitted<DELIM, T, COUNT> {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Delimitted {
            value: s.to_string(),
            cursor: 0,
            _t: PhantomData,
        })
    }
}

impl<const DELIM: char, const COUNT: usize, T: FromStr + Send + Sync + 'static> Iterator for Delimitted<DELIM, T, COUNT> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if COUNT == 0 {
            let mut value = self.value.split(DELIM).skip(self.cursor);
            self.cursor += 1;
            value.next().and_then(|v| T::from_str(v.trim()).ok())
        } else {
            let mut value = self.value.splitn(COUNT, DELIM).skip(self.cursor);
            self.cursor += 1;
            value.next().and_then(|v| T::from_str(v.trim()).ok())
        }
    }
}

impl<const DELIM: char, const COUNT: usize> FromStr for DelimittedStr<DELIM, COUNT> {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(DelimittedStr {
            value: s.to_string(),
            cursor: AtomicUsize::new(0),
        })
    }
}

impl<const DELIM: char, const COUNT: usize> DelimittedStr<DELIM, COUNT> {
    /// Returns the next value
    fn increment_counter(&self) {
        self.cursor
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Returns the current value
    fn value<'a: 'b, 'b>(&'a self) -> std::option::Option<Cow<'b, str>> {
        if COUNT == 0 {
            self.value
                .split(DELIM)
                .nth(self.cursor.load(std::sync::atomic::Ordering::Relaxed))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(Cow::Borrowed)
        } else {
            self.value
                .splitn(COUNT, DELIM)
                .nth(self.cursor.load(std::sync::atomic::Ordering::Relaxed))
                // .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(Cow::Borrowed)
        }
    }
}

impl<'a, const DELIM: char, const COUNT: usize> Iterator for &'a DelimittedStr<DELIM, COUNT> {
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


impl<const DELIM1: char, const DELIM2: char, const COUNT: usize> FromStr for DelimittedStr2<DELIM1, DELIM2, COUNT> {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(DelimittedStr2 {
            value: s.to_string(),
            cursor: AtomicUsize::new(0),
        })
    }
}

impl<const DELIM1: char, const DELIM2: char, const COUNT: usize> DelimittedStr2<DELIM1, DELIM2, COUNT> {
    /// Returns the next value
    fn increment_counter(&self) {
        self.cursor
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Returns the current value
    fn value<'a: 'b, 'b>(&'a self) -> std::option::Option<Cow<'b, str>> {
        if COUNT == 0 {
            self.value
                .split(&format!("{DELIM1}{DELIM2}"))
                .nth(self.cursor.load(std::sync::atomic::Ordering::Relaxed))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(Cow::Borrowed)
        } else {
            self.value
                .splitn(COUNT, &format!("{DELIM1}{DELIM2}"))
                .nth(self.cursor.load(std::sync::atomic::Ordering::Relaxed))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(Cow::Borrowed)
        }
    }
}

impl<'a, const DELIM1: char, const DELIM2: char, const COUNT: usize> Iterator for &'a DelimittedStr2<DELIM1, DELIM2, COUNT> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.value.contains(&format!("{DELIM1}{DELIM2}")) {
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

impl<const DELIM: char, T: FromStr + Send + Sync + 'static, const COUNT: usize> Default for Delimitted<DELIM, T, COUNT> {
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
    let mut results = scan_for_headers("Accept=text;;Names=test,abc;;");
    assert_eq!(("Accept", vec!["text"]), results.next().unwrap());
    assert_eq!(("Names", vec!["test", "abc"]), results.next().unwrap());
}
