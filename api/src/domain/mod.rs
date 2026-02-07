//! Pure bookkeeping logic: splitting amounts, computing balances and
//! suggesting settle-up transfers. Nothing in here touches the database, so
//! it is covered by plain unit tests.

pub mod balance;
pub mod category;
pub mod currency;
pub mod money;
pub mod simplify;
pub mod split;
