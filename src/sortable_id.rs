use serde::*;

use crate::date_time::DateTimeAsMicroseconds;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SortableId(String);

impl SortableId {
    pub fn generate() -> Self {
        use std::fmt::Write;
        let now = DateTimeAsMicroseconds::now();

        let mut result = now.unix_microseconds.to_string();

        write! {
            &mut result,
            "-{}",

            uuid::Uuid::new_v4()
        }
        .unwrap();

        result.truncate(25);

        Self(result)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Into<SortableId> for String {
    fn into(self) -> SortableId {
        SortableId(self)
    }
}

impl Into<String> for SortableId {
    fn into(self) -> String {
        self.0
    }
}

impl std::fmt::Display for SortableId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};

    use crate::SortableId;

    #[derive(Serialize, Deserialize)]
    pub struct TestStruct {
        pub id: SortableId,
    }

    #[test]
    fn test_generation() {
        let id = SortableId::generate();

        println!("{}", id);
    }

    #[test]
    fn test_serialize_deserialize() {
        let src = TestStruct {
            id: SortableId::generate(),
        };

        let payload = serde_json::to_string(&src).unwrap();

        println!("{}", payload);

        let dest: TestStruct = serde_json::from_str(&payload).unwrap();

        assert_eq!(src.id.as_str(), dest.id.as_str());
    }
}
