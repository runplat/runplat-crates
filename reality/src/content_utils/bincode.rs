use serde::Serialize;
use uuid::Uuid;

/// Content implementation that serializes the resource using `bincode` to generate
/// a state_uuid when the resource is put into state
pub struct BincodeContent {
    state_uuid: Uuid,
}

impl BincodeContent {
    /// Creates a new Bincode Content
    pub fn new<S: Serialize>(c: &S) -> std::io::Result<Self> {
        match bincode::serialize(c) {
            Ok(b) => {
                let mut crc = crate::content::crc().digest();
                crc.update(&b);
                Ok(Self {
                    state_uuid: uuid::Uuid::from_u64_pair(crc.finalize(), 0),
                })
            }
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                e.to_string(),
            )),
        }
    }
}

impl<S: Serialize> From<&S> for BincodeContent {
    fn from(value: &S) -> Self {
        Self::new(value).expect("should be able to create")
    }
}

impl runir::Content for BincodeContent {
    fn state_uuid(&self) -> uuid::Uuid {
        self.state_uuid.clone()
    }
}