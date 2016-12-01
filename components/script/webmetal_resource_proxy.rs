use canvas_traits::WebMetalDeviceRequest;
use ipc_channel::ipc::{self, IpcSender};
use std::collections::btree_map::{Entry, BTreeMap};
use webmetal;

pub type PassData = (webmetal::RenderPass, webmetal::FrameBuffer, webmetal::FrameClearData);

pub struct WebMetalResourceProxy {
    ipc_device: IpcSender<WebMetalDeviceRequest>,
    passes: BTreeMap<webmetal::TargetSet, PassData>,
}

impl WebMetalResourceProxy {
    pub fn new(ipc: IpcSender<WebMetalDeviceRequest>) -> WebMetalResourceProxy {
        WebMetalResourceProxy {
            ipc_device: ipc,
            passes: BTreeMap::new(),
        }
    }

    pub fn make_command_buffer(&self) -> webmetal::CommandBuffer {
        let (sender, receiver) = ipc::channel().unwrap();
        let req = WebMetalDeviceRequest::MakeCommandBuffer(sender);
        self.ipc_device.send(req).unwrap();
        receiver.recv().unwrap().unwrap()
    }

    pub fn make_render_pass(&mut self, targets: webmetal::TargetSet) -> PassData {
        match self.passes.entry(targets) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let (sender, receiver) = ipc::channel().unwrap();
                let req = WebMetalDeviceRequest::MakeRenderPass(sender, entry.key().clone());
                self.ipc_device.send(req).unwrap();
                let data = receiver.recv().unwrap().unwrap();
                entry.insert(data).clone()
            }
        }
    }
}
