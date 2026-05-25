use crate::board::Move;

#[derive(Copy, Clone, PartialEq)]
pub enum NodeType {
    Exact, // The score is the exact evaluation
    Alpha, // The score is an upper bound (failed low)
    Beta,  // The score is a lower bound (failed high)
}

#[derive(Copy, Clone)]
pub struct TTEntry {
    pub hash: u64,
    pub depth: u8,
    pub score: i32,
    pub node_type: NodeType,
    pub best_move: Option<Move>,
}

pub struct TranspositionTable {
    entries: Vec<Option<TTEntry>>,
    size: usize,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        // Calculate how many entries fit in the given Megabytes
        let entry_size = std::mem::size_of::<Option<TTEntry>>();
        let num_entries = (size_mb * 1024 * 1024) / entry_size;

        Self {
            entries: vec![None; num_entries],
            size: num_entries,
        }
    }

    pub fn probe(&self, hash: u64) -> Option<TTEntry> {
        let index = (hash as usize) % self.size;
        if let Some(entry) = self.entries[index] {
            // ALWAYS verify the full hash to avoid collisions!
            if entry.hash == hash {
                return Some(entry);
            }
        }
        None
    }

    pub fn store(
        &mut self,
        hash: u64,
        depth: u8,
        score: i32,
        node_type: NodeType,
        best_move: Option<Move>,
    ) {
        let index = (hash as usize) % self.size;

        self.entries[index] = Some(TTEntry {
            hash,
            depth,
            score,
            node_type,
            best_move,
        });
    }
}
