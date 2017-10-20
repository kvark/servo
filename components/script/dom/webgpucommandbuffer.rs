/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{
    WebGpuCommand, WebGpuCommandChan,
    BufferBarrier, ImageBarrier,
    CommandBufferId, CommandBufferInfo, CommandPoolId, SubmitEpoch, SubmitInfo,
    hal,
};
use dom::bindings::codegen::Bindings::WebGpuCommandBufferBinding as binding;
use dom::bindings::codegen::Bindings::WebGpuDeviceBinding::{WebGpuImageLayout, WebGpuPipelineStage};
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webgpubuffer::WebGpuBuffer;
use dom::webgpudescriptorset::WebGpuDescriptorSet;
use dom::webgpudevice::WebGpuDevice;
use dom::webgpuframebuffer::WebGpuFramebuffer;
use dom::webgpugraphicspipeline::WebGpuGraphicsPipeline;
use dom::webgpuimage::WebGpuImage;
use dom::webgpupipelinelayout::WebGpuPipelineLayout;
use dom::webgpurenderpass::WebGpuRenderPass;
use dom_struct::dom_struct;
use std::cell::Cell;


#[dom_struct]
pub struct WebGpuCommandBuffer {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuCommandChan,
    pool_id: CommandPoolId,
    info: CommandBufferInfo,
    submit_epoch: Cell<SubmitEpoch>, //TODO: atomics
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

    pub fn get_id(&self) -> CommandBufferId {
        self.info.id
    }

    pub fn to_submit_info(&self) -> SubmitInfo {
        SubmitInfo {
            pool_id: self.pool_id,
            cb_id: self.info.id,
            submit_epoch: self.submit_epoch.get(),
        }
    }

    fn map_buffer_state(state: binding::WebGpuBufferState) -> hal::buffer::State {
        let access = hal::buffer::Access::from_bits(state.access as _).unwrap();
        access
    }

    fn map_image_state(state: binding::WebGpuImageState) -> hal::image::State {
        let layout = WebGpuDevice::map_image_layout(state.layout);
        let access = hal::image::Access::from_bits(state.access as _).unwrap();
        (access, layout)
    }

    fn map_rect(rect: &binding::WebGpuRectangle) -> hal::target::Rect {
        hal::target::Rect {
            x: rect.x,
            y: rect.y,
            w: rect.width,
            h: rect.height,
        }
    }

    fn map_clear_value(cv: binding::WebGpuClearValue) -> hal::command::ClearValue {
        use self::binding::WebGpuClearValueKind::*;
        use self::hal::command::{ClearColor, ClearDepthStencil, ClearValue};
        match cv.kind {
            ColorUint => {
                let data = [*cv.data[0] as u32, *cv.data[1] as u32, *cv.data[2] as u32, *cv.data[3] as u32];
                ClearValue::Color(ClearColor::Uint(data))
            }
            ColorInt => {
                let data = [*cv.data[0] as i32, *cv.data[1] as i32, *cv.data[2] as i32, *cv.data[3] as i32];
                ClearValue::Color(ClearColor::Int(data))
            }
            ColorFloat => {
                let data = [*cv.data[0] as f32, *cv.data[1] as f32, *cv.data[2] as f32, *cv.data[3] as f32];
                ClearValue::Color(ClearColor::Float(data))
            }
            DepthStencil => {
                ClearValue::DepthStencil(ClearDepthStencil {
                    depth: *cv.data[0] as f32,
                    stencil: *cv.data[1] as u32,
                })
            }
        }
    }

    fn map_buffer_image_copy(copy: binding::WebGpuBufferImageCopy) -> hal::command::BufferImageCopy {
        hal::command::BufferImageCopy {
            buffer_offset: copy.bufferOffset as _,
            buffer_row_pitch: copy.bufferRowPitch as _,
            buffer_slice_pitch: copy.bufferSlicePitch as _,
            image_layers: hal::image::SubresourceLayers {
                aspects: hal::image::ASPECT_COLOR, //TODO
                level: 0,
                layers: 0 .. 1,
            },
            image_offset: hal::command::Offset {
                x: copy.imageOffset.x as _,
                y: copy.imageOffset.y as _,
                z: copy.imageOffset.z as _,
            },
            image_extent: hal::device::Extent {
                width: copy.imageExtent.width,
                height: copy.imageExtent.height,
                depth: copy.imageExtent.depth,
            },
        }
    }
}

impl binding::WebGpuCommandBufferMethods for WebGpuCommandBuffer {
    fn Begin(&self) {
        //TODO: remember if we are actively recording
        let msg = WebGpuCommand::Begin(self.info.id);
        self.sender.send(msg).unwrap();
    }

    fn Finish(&self) {
        let submit_epoch = self.submit_epoch.get() + 1;
        self.submit_epoch.set(submit_epoch); //TODO

        let msg = WebGpuCommand::Finish(submit_epoch);
        self.sender.send(msg).unwrap();
    }

    fn PipelineBarrier(
        &self,
        src_stages: WebGpuPipelineStage,
        dst_stages: WebGpuPipelineStage,
        buffers: Vec<binding::WebGpuBufferBarrier>,
        images: Vec<binding::WebGpuImageBarrier>,
    ) {
        let buffer_bars = buffers
            .into_iter()
            .map(|bar| BufferBarrier {
                states: Self::map_buffer_state(bar.stateSrc) ..
                        Self::map_buffer_state(bar.stateDst),
                target: bar.target.get_id(),
            })
            .collect();
        let image_bars = images
            .into_iter()
            .map(|bar| ImageBarrier {
                states: Self::map_image_state(bar.stateSrc) ..
                        Self::map_image_state(bar.stateDst),
                target: bar.target.get_id(),
            })
            .collect();

        let msg = WebGpuCommand::PipelineBarrier {
            stages: hal::pso::PipelineStage::from_bits(src_stages as _).unwrap() ..
                    hal::pso::PipelineStage::from_bits(dst_stages as _).unwrap(),
            buffer_bars,
            image_bars,
        };
        self.sender.send(msg).unwrap();
    }

    fn CopyBufferToImage(
        &self,
        source: &WebGpuBuffer,
        dest: &WebGpuImage,
        dest_layout: WebGpuImageLayout,
        regions: Vec<binding::WebGpuBufferImageCopy>,
    ) {
        let msg = WebGpuCommand::CopyBufferToImage {
            source_id: source.get_id(),
            dest_id: dest.get_id(),
            dest_layout: WebGpuDevice::map_image_layout(dest_layout),
            regions: regions
                .into_iter()
                .map(Self::map_buffer_image_copy)
                .collect(),
        };
        self.sender.send(msg).unwrap();
    }

    fn CopyImageToBuffer(
        &self,
        source: &WebGpuImage,
        source_layout: WebGpuImageLayout,
        dest: &WebGpuBuffer,
        regions: Vec<binding::WebGpuBufferImageCopy>,
    ) {
        let msg = WebGpuCommand::CopyImageToBuffer {
            source_id: source.get_id(),
            source_layout: WebGpuDevice::map_image_layout(source_layout),
            dest_id: dest.get_id(),
            regions: regions
                .into_iter()
                .map(Self::map_buffer_image_copy)
                .collect(),
        };
        self.sender.send(msg).unwrap();
    }

    fn BeginRenderPass(
        &self,
        render_pass: &WebGpuRenderPass,
        framebuffer: &WebGpuFramebuffer,
        rect: &binding::WebGpuRectangle,
        clearValues: Vec<binding::WebGpuClearValue>,
    ) {
        let clear_values = clearValues
            .into_iter()
            .map(Self::map_clear_value)
            .collect();

        let msg = WebGpuCommand::BeginRenderPass {
            render_pass: render_pass.get_id(),
            framebuffer: framebuffer.get_id(),
            area: Self::map_rect(rect),
            clear_values,
        };
        self.sender.send(msg).unwrap();
    }

    fn EndRenderPass(&self) {
        let msg = WebGpuCommand::EndRenderPass;
        self.sender.send(msg).unwrap();
    }

    fn BindGraphicsPipeline(&self, pso: &WebGpuGraphicsPipeline) {
        let msg = WebGpuCommand::BindGraphicsPipeline(pso.get_id());
        self.sender.send(msg).unwrap();
    }

    fn BindGraphicsDescriptorSets(
        &self,
        layout: &WebGpuPipelineLayout,
        desc_offset: u32,
        desc_sets: Vec<Root<WebGpuDescriptorSet>>,
    ) {
        let set_ids = desc_sets
            .into_iter()
            .map(|set| set.get_id())
            .collect();

        let msg = WebGpuCommand::BindGraphicsDescriptorSets {
            layout_id: layout.get_id(),
            desc_offset: desc_offset as _,
            set_ids,
        };
        self.sender.send(msg).unwrap();
    }

    fn SetScissors(&self, rectangles: Vec<binding::WebGpuRectangle>) {
        let rects = rectangles
            .iter()
            .map(Self::map_rect)
            .collect();

        let msg = WebGpuCommand::SetScissors(rects);
        self.sender.send(msg).unwrap();
    }

    fn SetViewports(&self, viewports: Vec<binding::WebGpuViewport>) {
        let ports = viewports
            .into_iter()
            .map(|vp| hal::Viewport::from_rect(Self::map_rect(&vp.rect), *vp.near, *vp.far))
            .collect();

        let msg = WebGpuCommand::SetViewports(ports);
        self.sender.send(msg).unwrap();
    }

    fn Draw(&self,
        start_vertex: u32, vertex_count: u32,
        start_instance: u32, instance_count: u32,
    ) {
        let msg = WebGpuCommand::Draw(
            start_vertex .. start_vertex+vertex_count,
            start_instance .. start_instance+instance_count,
        );
        self.sender.send(msg).unwrap();
    }
}
