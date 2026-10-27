//! Token counts as a log reports them.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub read: u64,
    pub write: u64,
    pub reasoning: u64,
}

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct TimelineBucket {
    pub input: u64,
    pub output: u64,
    pub cache: u64,
    pub entries: u64,
}

impl TimelineBucket {
    pub fn add(&mut self, other: Self) {
        self.input += other.input;
        self.output += other.output;
        self.cache += other.cache;
        self.entries += other.entries;
    }
    pub fn subtract(&mut self, other: Self) {
        self.input -= other.input;
        self.output -= other.output;
        self.cache -= other.cache;
        self.entries -= other.entries;
    }
}

impl Usage {
    pub fn total(self) -> u64 {
        self.input + self.output + self.reasoning + self.read + self.write
    }
    pub fn add(&mut self, other: Self) {
        self.input += other.input;
        self.output += other.output;
        self.read += other.read;
        self.write += other.write;
        self.reasoning += other.reasoning;
    }
    pub fn max(self, other: Self) -> Self {
        Self {
            input: self.input.max(other.input),
            output: self.output.max(other.output),
            read: self.read.max(other.read),
            write: self.write.max(other.write),
            reasoning: self.reasoning.max(other.reasoning),
        }
    }
    pub fn json(self) -> Value {
        json!({"input":self.input,"output":self.output,"cacheRead":self.read,"cacheWrite":self.write,"reasoning":self.reasoning,"total":self.total(),"usd":null,"cny":null})
    }
}

pub fn number(value: &Value, key: &str) -> u64 {
    value[key].as_u64().unwrap_or(0)
}
