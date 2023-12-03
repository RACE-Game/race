//! Module for calculating the merkel root of the to-be-uploaded file
//! This mod corresponds to the `lib/merkle.ts` module of arweave-js
use crate::arweave::crypto;
use crate::arweave::error::{Error, Result};
use borsh::BorshDeserialize;

const MAX_CHUNK_SIZE: usize = 256 * 1024;
const MIN_CHUNK_SIZE: usize = 32 * 1024;
const HASH_SIZE: usize = 32;
const NOTE_SIZE: usize = 32;

/// Includes a function to convert a number to a Vec of 32 bytes per the Arweave spec
trait Helpers<T> {
    fn to_note_vec(&self) -> Vec<u8>;
}

impl Helpers<usize> for usize {
    fn to_note_vec(&self) -> Vec<u8> {
        let mut note = vec![0; NOTE_SIZE - 8];
        note.extend((*self as u64).to_be_bytes());
        note
    }
}

/// Leaf node (data chunk) or branch node (hashes of paired child nodes)
#[derive(Debug, PartialEq, Clone)]
pub struct Node {
    pub id: [u8; HASH_SIZE],
    pub data_hash: Option<[u8; HASH_SIZE]>,
    pub min_byte_range: usize,
    pub max_byte_range: usize,
    pub left_child: Option<Box<Node>>,
    pub right_child: Option<Box<Node>>,
}

/// Concatenated ids and offsets for full set of nodes for an original data chunk, starting with the root.
#[derive(Debug, PartialEq, Clone)]
pub struct Proof {
    pub offset: usize,
    proof: Vec<u8>,
}

impl Proof {
    pub fn proof(&self) -> Vec<u8> {
        self.proof.clone()
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

}


/// Populated with data from deserialized [`Proof`] for original data chunk (Leaf [`Node`])
#[repr(C)]
#[derive(BorshDeserialize, Debug, PartialEq, Clone)]
pub struct LeafProof {
    data_hash: [u8; HASH_SIZE],
    notepad: [u8; NOTE_SIZE - 8],
    offset: [u8; 8],
}

/// Populated with data from deserialized [`Proof`] for branch [`Node`] (hash of paired child nodes)
#[derive(BorshDeserialize, Debug, PartialEq, Clone)]
pub struct BranchProof {
    left_id: [u8; HASH_SIZE],
    right_id: [u8; HASH_SIZE],
    notepad: [u8; NOTE_SIZE - 8],
    offset: [u8; 8],
}

/// Includes methods to deserialize [`Proof`]s
pub trait ProofDeserialize<T> {
    fn try_from_proof_slice(slice: &[u8]) -> Result<T>;
    fn offset(&self) -> usize;
}

impl ProofDeserialize<LeafProof> for LeafProof {
    fn try_from_proof_slice(slice: &[u8]) -> Result<Self> {
        let proof = LeafProof::try_from_slice(slice)?;
        Ok(proof)
    }
    fn offset(&self) -> usize {
        usize::from_be_bytes(self.offset)
    }
}

impl ProofDeserialize<BranchProof> for BranchProof {
    fn try_from_proof_slice(slice: &[u8]) -> Result<Self> {
        let proof = BranchProof::try_from_slice(slice)?;
        Ok(proof)
    }
    fn offset(&self) -> usize {
        usize::from_be_bytes(self.offset)
    }
}

/// Split data into chunks for generating tree leaves.  This is a literal
/// translation of arweave-js `chunkData` and not currently in use.
#[allow(dead_code)]
fn chunk_data(data: Vec<u8>) -> Result<Vec<Vec<u8>>> {
    let mut chunks = Vec::<Vec<u8>>::new();
    let mut cursor = 0usize;

    let mut rest = data.clone();
    while rest.len() >= MAX_CHUNK_SIZE {
        #[allow(dead_code)]
        let mut chunk_size = MAX_CHUNK_SIZE;
        let next_chunk_size = rest.len() - MAX_CHUNK_SIZE;

        if next_chunk_size > 0 && next_chunk_size < MIN_CHUNK_SIZE {
            chunk_size = if rest.len() % 2 == 0 {
                rest.len() / 2
            } else {
                rest.len() / 2 + 1
            }
        }

        let chunk = rest[0..chunk_size].to_vec();
        cursor += chunk.len();
        println!("cursor now at {}", cursor);
        chunks.push(chunk);
        rest = rest[chunk_size..].to_vec();
    }

    chunks.push(rest);
    Ok(chunks)
}

/// Generates leaves from which the calculation of root id starts
pub fn generate_leaves(data: Vec<u8>) -> Result<Vec<Node>> {


    let mut data_chunks: Vec<&[u8]> = data.chunks(MAX_CHUNK_SIZE).collect();

    #[allow(unused_assignments)]
    let mut last_two = Vec::new();

    // If the total bytes left will produce a chunk < MIN_CHUNK_SIZE,
    // then adjust the rest amount by cutting it into halves as equally as possible.
    if data_chunks.len() > 1 && data_chunks.last().unwrap().len() < MIN_CHUNK_SIZE {
        last_two = data_chunks.split_off(data_chunks.len() - 2).concat();

        let chunk_size = if last_two.len() % 2 != 0 {
            last_two.len() / 2 + 1
        } else {
            last_two.len() / 2
        };

        data_chunks.append(&mut last_two.chunks(chunk_size).collect::<Vec<&[u8]>>());
    }

    if data_chunks.last().unwrap().len() == MAX_CHUNK_SIZE {
        data_chunks.push(&[]);
    }

    let mut leaves = Vec::<Node>::new();
    let mut min_byte_range = 0;

    // Hash each chunk twice according to the arweave spec:
    // first hash the chunck itself and then hash the concat of chunk hash + offset
    for chunk in data_chunks.into_iter() {
        let data_hash = crypto::sha256_hash(&chunk)?;
        let max_byte_range = min_byte_range + chunk.len();
        let offset = max_byte_range.to_note_vec();
        let concat_hashes = vec![&data_hash, &offset[..]];
        let id = crypto::sha256_hash_all(concat_hashes)?;

        leaves.push(Node {
            id,
            data_hash: Some(data_hash),
            min_byte_range,
            max_byte_range,
            left_child: None,
            right_child: None,
        });
        min_byte_range = min_byte_range + chunk.len();
    }

    Ok(leaves)
}

/// Hashes a pair of child nodes and get single branch node
fn hash_branch(left: Node, right: Node) -> Result<Node> {
    let max_byte_range = left.max_byte_range.to_note_vec();
    let id = crypto::sha256_hash_all(vec![&left.id, &right.id, &max_byte_range])?;
    Ok(Node {
        id,
        data_hash: None, // branch node has no data hash
        min_byte_range: left.max_byte_range,
        max_byte_range: right.max_byte_range,
        left_child: Some(Box::new(left)),
        right_child: Some(Box::new(right)),
    })
}

/// Builds one layer of branch nodes from a layer of child nodes
pub fn build_layer<'a>(nodes: Vec<Node>) -> Result<Vec<Node>> {
    let cap = if nodes.len() % 2 != 0 {
        nodes.len() / 2 + 1
    } else {
        nodes.len() / 2
    };
    let mut layer = Vec::<Node>::with_capacity(cap);
    let mut nodes_iter = nodes.into_iter();
    while let Some(left) = nodes_iter.next() {
        if let Some(right) = nodes_iter.next() {
            layer.push(hash_branch(left, right)?);
        } else {
            layer.push(left);
        }
    }

    Ok(layer)
}

/// Builds all layers from leaves up to the top root node
pub fn generate_root(mut nodes: Vec<Node>) -> Result<Node> {
    while nodes.len() > 1 {
        nodes = build_layer(nodes)?;
    }
    nodes.pop().ok_or(Error::NoRootNodeFound)
}

/// Calculates [`Proof`] for each data chunk contained in root [`Node`]
/// Calculation starts at a branch level and goes down to child nodes
/// At each level, proof for left child node is recorded first
/// For a two-leaf-one-parent tree, the final proof vec looks like
/// [[{Loffest, [Lid,Rid,Off,Ldhash,LOff]}],[{Roffset, [Lid,Rid,Off,Rdhash,ROff]}]]
///
/// Given a leaf, its proof offset = its max_byte_range - 1 (< its max_byte_range)
/// Thus, when a byte_may_range is less than a proof offset, the max_byte_range
/// belongs to a node that is left to the current one
pub fn resolve_proofs(node: Node, proof: Option<Proof>) -> Result<Vec<Proof>> {
    let mut proof = if let Some(proof) = proof {
        proof
    } else {
        Proof {
            offset: 0,
            proof: Vec::<u8>::new(),
        }
    };
    match node {
        // Leaf (has data but no id or child nodes)
        Node {
            data_hash: Some(data_hash),
            max_byte_range,
            left_child: None,
            right_child: None,
            ..
        } => {
            proof.offset = max_byte_range - 1;
            proof.proof.extend(data_hash);
            proof.proof.extend(max_byte_range.to_note_vec());
            return Ok(vec![proof]);
        }
        // Branch (has child nodes but no data)
        Node {
            data_hash: None,
            min_byte_range,
            left_child: Some(left_child),
            right_child: Some(right_child),
            ..
        } => {
            // Record left/right child ids in proof
            proof.proof.extend(left_child.id.clone());
            proof.proof.extend(right_child.id.clone());
            proof.proof.extend(min_byte_range.to_note_vec());

            // Go down one level
            let mut left_proof = resolve_proofs(*left_child, Some(proof.clone()))?;
            let right_proof = resolve_proofs(*right_child, Some(proof))?;
            left_proof.extend(right_proof);
            return Ok(left_proof);
        }
        _ => unreachable!(),
    }
}

/// Validates a specific chunk of data against provided [`Proof`]
#[allow(dead_code)]
fn validate_chunk(
    mut root_id: [u8; HASH_SIZE],
    chunk: Node,  // leaf node to be validated
    proof: Proof, // proof corresponding to the leaf/data chunk
) -> Result<()> {
    match chunk {
        Node {
            data_hash: Some(data_hash),
            max_byte_range,
            ..
        } => {
            // Split proof into branches and last leaf
            // Leaf is at the end and branches are ordered from root to leaves
            let (branches, leaf) = proof
                .proof
                .split_at(proof.proof.len() - HASH_SIZE - NOTE_SIZE);

            // Deserialize proofs
            let branch_proofs: Vec<BranchProof> = branches
                .chunks(HASH_SIZE * 2 + NOTE_SIZE)
                .map(|b| BranchProof::try_from_proof_slice(b).unwrap())
                .collect();
            let leaf_proof = LeafProof::try_from_proof_slice(leaf)?;

            // Validate branches
            for branch_proof in branch_proofs.iter() {
                // Calculate the id from the proof
                let id = crypto::sha256_hash_all(vec![
                    &branch_proof.left_id,
                    &branch_proof.right_id,
                    &branch_proof.offset().to_note_vec(),
                ])?;

                // Ensure calculated id correct
                if !(id == root_id) {
                    return Err(Error::InvalidProof.into());
                }

                // Update current root id to be one of child nodes
                // If proof offset is greater than data chunk offset,
                // then the next id to validate against is from the left
                root_id = match max_byte_range > branch_proof.offset() {
                    true => branch_proof.right_id,
                    false => branch_proof.left_id,
                }
            }

            // Validate leaf: both id and data_hash are correct
            let id = crypto::sha256_hash_all(vec![&data_hash, &max_byte_range.to_note_vec()])?;
            if !(id == root_id) & !(data_hash == leaf_proof.data_hash) {
                return Err(Error::InvalidProof.into());
            }
        }
        _ => {
            unreachable!()
        }
    }
    Ok(())
}
