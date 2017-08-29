/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{
    WebGpuCommand, WebGpuCommandChan,
    CommandBufferInfo, CommandPoolId, SubmitEpoch, SubmitInfo,
    BufferBarrier, BufferState, ImageBarrier, ImageState,
};
use dom::bindings::codegen::Bindings::WebGpuCommandBufferBinding as binding;
//use dom::bindings::codegen::Bindings::WebGpuDeviceBinding as dev_binding;
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

fn map_buffer_state(_state: binding::WebGpuBufferState) -> BufferState {
    unimplemented!()
}

fn map_image_state(_state: binding::WebGpuImageState) -> ImageState {
    unimplemented!()
}

impl binding::WebGpuCommandBufferMethods for WebGpuCommandBuffer {
    fn Finish(&self) -> Root<WebGpuSubmit> {
        let submit_epoch = self.submit_epoch.get() + 1;
        self.submit_epoch.set(submit_epoch); //TODO

        let msg = WebGpuCommand::Finish(self.info.id, submit_epoch);
        self.sender.send(msg).unwrap();

        let info = SubmitInfo {
            pool_id: self.pool_id,
            cb_id: self.info.id,
            submit_epoch,
        };
        WebGpuSubmit::new(&self.global(), info)
    }

    fn PipelineBarrier(&self,
        buffer_bars: Vec<binding::WebGpuBufferBarrier>,
        image_bars: Vec<binding::WebGpuImageBarrier>,
    ) {
        let buffers = buffer_bars
            .into_iter()
            .map(|bar| BufferBarrier {
                state_src: map_buffer_state(bar.stateSrc),
                state_dst: map_buffer_state(bar.stateDst),
                target: bar.target.get_id(),
            })
            .collect();
        let images = image_bars
            .into_iter()
            .map(|bar| ImageBarrier {
                state_src: map_image_state(bar.stateSrc),
                state_dst: map_image_state(bar.stateDst),
                target: bar.target.get_id(),
            })
            .collect();

        let msg = WebGpuCommand::PipelineBarrier(buffers, images);
        self.sender.send(msg).unwrap();
    }
}

impl Drop for WebGpuCommandBuffer {
    fn drop(&mut self) {
        let msg = WebGpuCommand::ReturnCommandBuffer(self.info.id);
        self.sender.send(msg).unwrap();
    }
}
