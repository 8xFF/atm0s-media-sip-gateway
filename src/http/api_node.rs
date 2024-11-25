use atm0s_small_p2p::PeerAddress;
use poem_openapi::{payload::PlainText, OpenApi};

pub struct NodeApiCtx {
    pub address: PeerAddress,
}

pub struct Apis {
    ctx: NodeApiCtx,
}

impl Apis {
    pub fn new(ctx: NodeApiCtx) -> Self {
        Self { ctx }
    }
}

#[OpenApi]
impl Apis {
    #[oai(path = "/address", method = "get")]
    async fn get_address(&self) -> PlainText<String> {
        PlainText(self.ctx.address.to_string())
    }
}
