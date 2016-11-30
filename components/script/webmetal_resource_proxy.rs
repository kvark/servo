use canvas_traits::{CanvasMsg, WebMetalCommand};
use ipc_channel::ipc::{self, IpcSender};
use std::collections::btree_map::{Entry, BTreeMap};
use webmetal;

pub type PassData = (webmetal::RenderPass, webmetal::FrameBuffer, webmetal::FrameClearData);

pub struct WebMetalResourceProxy {
    //TODO: change to the Device-only sender
    ipc_device: IpcSender<CanvasMsg>,
    passes: BTreeMap<webmetal::TargetSet, PassData>,
}

impl WebMetalResourceProxy {
    pub fn new(ipc: IpcSender<CanvasMsg>) -> WebMetalResourceProxy {
        WebMetalResourceProxy {
            ipc_device: ipc,
            passes: BTreeMap::new(),
        }
    }

    pub fn make_render_pass(&mut self, targets: webmetal::TargetSet) -> PassData {
        match self.passes.entry(targets) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let (sender, receiver) = ipc::channel().unwrap();
                let msg = WebMetalCommand::MakeRenderPass(sender, entry.key().clone());
                self.ipc_device.send(CanvasMsg::WebMetal(msg)).unwrap();
                let data = receiver.recv().unwrap().unwrap();
                entry.insert(data).clone()
            }
        }
    }
}
