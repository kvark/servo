/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{
    WebGpuCommand, WebGpuCommandChan, CommandPoolId, CommandPoolInfo,
    webgpu_channel,
};
use dom::bindings::codegen::Bindings::WebGpuCommandPoolBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webgpucommandbuffer::WebGpuCommandBuffer;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuCommandPool {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuCommandChan,
    id: CommandPoolId,
}

impl WebGpuCommandPool {
    pub fn new(
        global: &GlobalScope,
        info: CommandPoolInfo,
    ) -> Root<Self>
    {
        let obj = box WebGpuCommandPool {
            reflector_: Reflector::new(),
            sender: info.channel,
            id: info.id,
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}

impl binding::WebGpuCommandPoolMethods for WebGpuCommandPool {
    fn Reset(&self) {
        let msg = WebGpuCommand::Reset;
        self.sender.send(msg).unwrap()
    }

    fn AcquireCommandBuffer(&self) -> Root<WebGpuCommandBuffer> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuCommand::AcquireCommandBuffer(sender);
        self.sender.send(msg).unwrap();
        let combuf = receiver.recv().unwrap();
        WebGpuCommandBuffer::new(&self.global(), self.sender.clone(), self.id, combuf)
    }
}
