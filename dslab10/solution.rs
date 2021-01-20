use actix::prelude::*;
use uuid::Uuid;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum ProductType {
    Electronics,
    Toys,
    Books,
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
/// Add any fields you might need.
pub(crate) struct TwoPhaseCommit {}

#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct TransactionMessage {
    pub(crate) transaction: Transaction,
    /// Called after 2PC commit completes, that is either Commit or Abort was decided
    /// by CyberStore2047. This must be called after responses from all processes
    /// acknowledging commit or abort are collected.
    pub(crate) completed_callback: Box<dyn FnOnce(TwoPhaseResult) + Send>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum TwoPhaseResult {
    Ok,
    Abort,
}

#[derive(Copy, Clone)]
pub(crate) struct Product {
    pub(crate) identifier: Uuid,
    pub(crate) pr_type: ProductType,
    /// Price in smallest monetary unit.
    pub(crate) price: u64,
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Transaction {
    pub(crate) pr_type: ProductType,
    pub(crate) shift: i32,
}

#[derive(Message)]
#[rtype(result = "ProductPrice")]
pub(crate) struct ProductPriceQuery {
    pub(crate) product_ident: Uuid,
}

pub(crate) struct ProductPrice(pub(crate) Option<u64>);

/// Add any fields you might need. This structure will serve as TM.
pub(crate) struct CyberStore2047 {}

impl CyberStore2047 {
    pub(crate) fn new(nodes: Vec<Recipient<TwoPhaseCommit>>) -> Self {
        unimplemented!()
    }
}

pub(crate) struct Node {
    products: Vec<Product>,
    pending_transaction: Option<Transaction>,
}

impl Node {
    pub(crate) fn new(products: Vec<Product>) -> Self {
        unimplemented!()
    }
}

impl Actor for CyberStore2047 {
    type Context = Context<Self>;
}

impl Actor for Node {
    type Context = Context<Self>;
}

impl Handler<TwoPhaseCommit> for CyberStore2047 {
    type Result = ();

    fn handle(&mut self, msg: TwoPhaseCommit, ctx: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}

impl Handler<TwoPhaseCommit> for Node {
    type Result = ();

    fn handle(&mut self, msg: TwoPhaseCommit, ctx: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}

impl Handler<ProductPriceQuery> for Node {
    type Result = MessageResult<ProductPriceQuery>;

    fn handle(&mut self, msg: ProductPriceQuery, _: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}

impl Handler<TransactionMessage> for CyberStore2047 {
    type Result = ();

    fn handle(&mut self, msg: TransactionMessage, ctx: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}
