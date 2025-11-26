//! Pure bookkeeping logic: splitting amounts, computing balances and
//! suggesting settle-up transfers. Nothing in here touches the database, so
//! it is covered by plain unit tests.

pub mod currency;
