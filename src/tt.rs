use crate::board::Move;

#[derive(Default, Debug, Clone, Copy)]
pub struct TTEntry {
    pub hash: u64,
    pub depth: u8,
    pub score: i32,
    pub best_move: Move,
}

#[derive(Debug, Default, Clone)]
pub struct TT {
    entries: Vec<TTEntry>,
}

impl TT {
    pub fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TTEntry>();
        let num_entries = (size_mb * 1024 * 1024) / entry_size;
        Self {
            entries: vec![TTEntry::default(); num_entries],
        }
    }

    pub fn is_empty(&self, idx: usize) -> bool {
        self.entries[idx].hash == 0
    }

    pub fn get_index(&self, hash: u64) -> usize {
        (hash as usize) % self.entries.len()
    }

    pub fn add_entry(&mut self, entry: TTEntry) {
        let idx = (entry.hash as usize) % self.entries.len();

        if self.entries[idx].hash != entry.hash || self.entries[idx].depth < entry.depth {
            self.entries[idx] = entry;
        }
    }

    pub fn get_entry(&self, idx: usize) -> TTEntry {
        self.entries[idx]
    }
}
