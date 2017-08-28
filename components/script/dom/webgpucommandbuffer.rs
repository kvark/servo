/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{
    WebGpuCommand, WebGpuCommandChan,
    CommandBufferInfo, CommandPoolId, SubmitEpoch, SubmitInfo,
};
use dom::bindings::codegen::Bindings::WebGpuCommandBufferBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webgpusubmit::WebGpuSubmit;
use dom_struct::dom_struct;
use std::cell::Cell;


#[dom_struct]
pub struct WebGpuCommandBuffer {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuCommandChan,
    pool_id: CommandPoolId,
    info: CommandBufferInfo,
    submit_epoch: Cell<SubmitEpoch>,
}

impl WebGpuCommandBuffer {
    pub fn new(
        global: &GlobalScope,
        sender: WebGpuCommandChan,
        pool_id: CommandPoolId,
        info: CommandBufferInfo,
    ) -> Root<Self> {
        let obj = box WebGpuCommandBuffer {
            reflector_: Reflector::new(),
            sender,
            pool_id,
            info,
            submit_epoch: Cell::new(0),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}

impl binding::WebGpuCommandBufferMethods for WebGpuCommandBuffer {
    fn Finish(&self) -> Root<WebGpuSubmit> {
        let submit_epoch = self.submit_epoch.get() + 1;
        self.submit_epoch.set(submit_epoch); //TODO
        let info = SubmitInfo {
            pool_id: self.pool_id,
            cb: self.info.clone(),
            submit_epoch,
        };
        WebGpuSubmit::new(&self.global(), info)
    }
}

impl Drop for WebGpuCommandBuffer {
    fn drop(&mut self) {
        let msg = WebGpuCommand::ReturnCommandBuffer(self.info.id);
        self.sender.send(msg).unwrap();
    }
}
