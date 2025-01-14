use crate::synchronizer::Synchronizer;
use crate::{Status, StatusCode};
use ckb_logger::info;
use ckb_network::{CKBProtocolContext, PeerIndex};

pub struct InIBDProcess<'a> {
    synchronizer: &'a Synchronizer,
    peer: PeerIndex,
    nc: &'a dyn CKBProtocolContext,
}

impl<'a> InIBDProcess<'a> {
    pub fn new(
        synchronizer: &'a Synchronizer,
        peer: PeerIndex,
        nc: &'a dyn CKBProtocolContext,
    ) -> Self {
        InIBDProcess {
            nc,
            synchronizer,
            peer,
        }
    }

    pub fn execute(self) -> Status {
        info!("getheader with ibd peer {:?}", self.peer);
        if let Some(mut kv_pair) = self.synchronizer.peers().state.get_mut(&self.peer) {
            let state = kv_pair.value_mut();

            // The node itself needs to ensure the validity of the outbound connection.
            //
            // If outbound is an ibd node(non-whitelist), it should be disconnected automatically.
            // If inbound is an ibd node, just mark the node does not pass header sync authentication.
            if state.peer_flags.is_outbound {
                if state.peer_flags.is_whitelist {
                    self.synchronizer.shared().state().suspend_sync(state);
                } else if let Err(err) = self.nc.disconnect(self.peer, "outbound in ibd") {
                    return StatusCode::Network
                        .with_context(format!("Disconnect error: {:?}", err));
                }
            } else {
                self.synchronizer.shared().state().suspend_sync(state);
            }
        }
        Status::ok()
    }
}
