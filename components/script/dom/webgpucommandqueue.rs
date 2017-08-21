/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{DeviceId, QueueId, WebGpuChan, WebGpuMsg, webgpu_channel};
use dom::bindings::codegen::Bindings::WebGpuCommandQueueBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webgpucommandpool::WebGpuCommandPool;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuCommandQueue {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuChan,
    id: (QueueId, DeviceId),
}

impl WebGpuCommandQueue {
    pub fn new(
        global: &GlobalScope,
        sender: WebGpuChan,
        id: QueueId,
        device_id: DeviceId,
    ) -> Root<Self> {
        let obj = box WebGpuCommandQueue {
            reflector_: Reflector::new(),
            sender,
            id: (id, device_id),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }

    pub fn device_id(&self) -> DeviceId {
        self.id.1
    }
}

impl binding::WebGpuCommandQueueMethods for WebGpuCommandQueue {
    fn CreateCommandPool(&self, max_buffers: u32) -> Root<WebGpuCommandPool> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateCommandPool {
            device_id: self.id.1,
            queue_id: self.id.0,
            max_buffers,
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuCommandPool::new(&self.global(), info.channel)
    }
}
