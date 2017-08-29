/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{GpuId, QueueId, WebGpuChan, WebGpuMsg, webgpu_channel};
use dom::bindings::codegen::Bindings::WebGpuCommandQueueBinding as binding;
use dom::bindings::codegen::Bindings::WebGpuDeviceBinding::{WebGpuFence, WebGpuSemaphore};
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webgpucommandpool::WebGpuCommandPool;
use dom::webgpusubmit::WebGpuSubmit;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuCommandQueue {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuChan,
    id: (GpuId, QueueId),
}

impl WebGpuCommandQueue {
    pub fn new(
        global: &GlobalScope,
        sender: WebGpuChan,
        gpu_id: GpuId,
        id: QueueId,
    ) -> Root<Self> {
        let obj = box WebGpuCommandQueue {
            reflector_: Reflector::new(),
            sender,
            id: (gpu_id, id),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }

    pub fn gpu_id(&self) -> GpuId {
        self.id.0
    }
}

impl binding::WebGpuCommandQueueMethods for WebGpuCommandQueue {
    fn CreateCommandPool(&self, max_buffers: u32) -> Root<WebGpuCommandPool> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateCommandPool {
            gpu_id: self.id.0,
            queue_id: self.id.1,
            max_buffers,
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuCommandPool::new(&self.global(), info)
    }

    fn Submit(&self,
        command_bufs: Vec<Root<WebGpuSubmit>>,
        _waits: Vec<WebGpuSemaphore>,
        _signals: Vec<WebGpuSemaphore>,
        _fence: WebGpuFence,
    ) {
        let msg = WebGpuMsg::Submit {
            gpu_id: self.id.0,
            queue_id: self.id.1,
            command_buffers: command_bufs
                .iter()
                .map(|cb| cb.to_info())
                .collect(),
            wait_semaphores: Vec::new(), //TODO
            signal_semaphores: Vec::new(), //TODO
            fence: None, //TODO
        };
        self.sender.send(msg).unwrap();
    }
}
