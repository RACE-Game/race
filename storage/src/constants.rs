pub const AR_KEYFILE_PATH: &str = "/home/noel/projects/wallets/arweave.json";
pub const MAX_RETRIES: u16 = 5;
pub const RETRY_SLEEP: u64 = 1; // second
/// Recommended way to get the anchor for the `last_tx` field of a transaction:
/// docs.arweave.org/developers/arweave-node-server/http-api#field-definitions
pub const AR_ANCHOR: &str = "tx_anchor";
/// Winston is the smallest unit of the native Arweave network token, AR
/// https://docs.arweave.org/developers/arweave-node-server/http-api#ar-and-winston
pub const WINSTONS_PER_AR: u64 = 1_000_000_000_000; // 12 zeros
/// Maximum data size to send to `tx/` endpoint.
/// Data above this size threshold should be sent to `chunk/` endpoint.
pub const MAX_TX_DATA: u64 = 10_000_000;
/// Race default logo as the bundle cover
pub const RACE_LOGO_URI: &str = "https://arweave.net/UtfjpKPm9HvJJ11WN3kL2EYdQ5zauCFE56D1CnSV9s4";

/// field length limits per the official source code:
/// https://github.com/metaplex-foundation/js/blob/5bfbd36921d0299f5013a67e2aedd1ae6a6cb2de/packages/js/src/plugins/candyMachineModule/constants.ts#L1-L5

pub const MAX_NAME_LENGTH: usize = 32;
pub const MAX_SYMBOL_LENGTH: usize = 10;
pub const MAX_URI_LENGTH: usize = 200;
pub const MAX_CREATOR_LIMIT: usize = 5;
pub const DEFAULT_METADATA_PTAH: &str = "metadata.json";
/// Block size used for pricing calculations = 256 KB
pub const BLOCK_SIZE: u64 = 1024 * 256;

// Wasm1 file size: 482675B and will be chunked to
// [(0B,262143B), (262144B,482675B)]
pub const WASM1: &str = "tests/race_holdem_cash1.wasm";
// Wasm2 file size: 524722B and will be chunked to
// [(0B,262144B), (262144B,393433), (393433,524722B)]
pub const WASM2: &str = "tests/race_holdem_cash2.wasm";
pub const WASM3: &str = "/home/noel/projects/racepoker/race-holdem/target/race_holdem_mtt.wasm";
