pub const SCALE: i128 = 100_000_000_000_000;
pub const ORACLE_ADDRESS: &str = "CBKZFI26PDCZUJ5HYYKVB5BWCNYUSNA5LVL4R2JTRVSOB4XEP7Y34OPN";
pub const ORACLE_FUNCTION: &str = "x_last_price";
pub const COLLATERAL_BUFFER: i128 = 20;
pub const COLLATERAL_THRESHOLD: i128 = 125;
pub const TIME_TO_EXEC: u64 = 86400;  // 86400sg = 12 hours
pub const TIME_TO_REPAY: u64 = 172800; // 172800sg = 48 hours

#[cfg(test)]
pub const TIME_TO_MATURE: u64 = 604800; // 604800sg = 1 week