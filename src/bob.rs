use anyhow::Result;
use rkyv::{Archive, Deserialize, Serialize, deserialize, rancor::Error, util::AlignedVec};


// lifetime?



#[derive(Archive, Deserialize, Serialize, Debug, PartialEq, Default, Clone)]
#[rkyv(
    // This will generate a PartialEq impl between our unarchived and archived types
    compare(PartialEq),
    // Derives can be passed through to the generated type:
    derive(Debug),
)]
pub struct Bob {
    pub count: i64,
    pub tag: String
}

pub trait Man {
    fn greet(&self);
}

impl Man for Bob {
    fn greet(&self) {
        println!("Greetings, {}, we count up to {}", self.count, self.tag);
    }
}

impl Bob {
    pub fn new(c: i64, t: String) -> Self {
        Bob { count: c, tag: t }
    }

    pub fn from_blob(bytes: AlignedVec) -> Result<Self> {
        let archived = rkyv::access::<ArchivedBob, Error>(&bytes[..])?;
        let deserialized = deserialize::<Bob, Error>(archived).unwrap();
        Ok(deserialized)
    }

    pub fn to_blob(&self) -> Result<AlignedVec> {
        Ok(rkyv::to_bytes::<Error>(self)?)
    }
}
