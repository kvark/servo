/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::cell::Cell;

use canvas_traits::webgpu::{WebGpuChan, WebGpuCommand, WebGpuMsg,
    gpu, webgpu_channel,
    BufferId, BufferDesc, CommandBufferId, CommandPoolInfo, ContextId,
    FenceId, GpuId, HeapId, HeapDesc, BufferBarrier, ImageBarrier,
    ImageDesc, SubmitEpoch, SubmitInfo, QueueId,
};
use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::WebGpuSwapchainBinding as binding;
use dom::bindings::codegen::Bindings::WebGpuCommandBufferBinding::{WebGpuImageState};
use dom::bindings::codegen::Bindings::WebGpuDeviceBinding::{WebGpuFormat, WebGpuFramebufferSize, WebGpuImageLayout};
use dom::bindings::js::{JS, Root};
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::node::{Node, NodeDamage};
use dom::webgpucommandqueue::WebGpuCommandQueue;
use dom::webgpudevice::WebGpuDevice;
use dom::webgpuimage::WebGpuImage;
use dom::webgpusemaphore::WebGpuSemaphore;
use dom_struct::dom_struct;
use euclid::Size2D;


pub struct IdRotation {
    total: binding::WebGpuSwapchainImageId,
    acquire: binding::WebGpuSwapchainImageId,
    present: Option<binding::WebGpuSwapchainImageId>,
}

impl IdRotation {
    fn new(total: binding::WebGpuSwapchainImageId) -> Self {
        IdRotation {
            total,
            acquire: 0,
            present: None,
        }
    }

    fn acquire(&mut self) -> Option<binding::WebGpuSwapchainImageId> {
        let id = self.acquire;
        if Some(id) != self.present {
            if self.present.is_none() {
                self.present = Some(id);
            }
            self.acquire += 1;
            if self.acquire >= self.total {
                self.acquire = 0;
            }
            Some(id)
        } else {
            None
        }
    }

    fn present(&mut self) -> Option<binding::WebGpuSwapchainImageId> {
        self.present
            .take()
            .map(|id| {
                let mut next = id + 1;
                if next >= self.total {
                    next = 0;
                }
                if next != self.acquire {
                    self.present = Some(next);
                }
                id
            })
    }
}

#[derive(HeapSizeOf)]
pub struct Frame {
    image: Root<WebGpuImage>,
    staging_buffer_id: BufferId,
    command_buffer_id: CommandBufferId,
    fence_id: FenceId,
}

// dummy wrapper in order to avoid `dom_struct` errors
#[derive(HeapSizeOf)]
pub struct WebGpuParent {
    context_id: ContextId,
    gpu_id: GpuId,
    queue_id: QueueId,
}

#[dom_struct]
pub struct WebGpuSwapchain {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuChan,
    canvas_node: JS<Node>,
    parent: WebGpuParent,
    #[ignore_heap_size_of = "Channels are hard"]
    command_pool_info: CommandPoolInfo,
    format: WebGpuFormat,
    size: Size2D<u32>,
    bytes_per_row: usize,
    bytes_per_image: usize,
    gpu_heap_id: HeapId,
    staging_heap_id: HeapId,
    frames: Vec<Frame>,
    present_epoch: Cell<SubmitEpoch>, //TODO: atomics
    #[ignore_heap_size_of = "Nothing to see here"]
    id_rotation: DOMRefCell<IdRotation>,
}

impl WebGpuSwapchain {
    pub fn new(
        global: &GlobalScope,
        sender: WebGpuChan,
        node: &Node,
        context_id: ContextId,
        queue: &WebGpuCommandQueue,
        count: usize,
        format: WebGpuFormat,
        size: Size2D<u32>,
    ) -> Root<Self>
    {
        //Note: this constructor is an exception from the rule
        // as it's making the send calls internally, as opposed to
        // receiving an `*Info` struct having all the inputs
        let align = |x: usize, to: usize| { (x + to - 1) & !(to - 1) };

        // rough upper bound for the image size
        let stride = 4usize;
        let limits = queue.get_limits();
        let bytes_per_row = align(size.width as usize * stride, limits.min_buffer_copy_pitch_alignment);
        let bytes_per_image = align(align(size.height as usize, 0x100) * bytes_per_row, 0x10000);

        let staging_heap_id = {
            let (result, receiver) = webgpu_channel().unwrap();
            let msg = WebGpuMsg::CreateHeap {
                gpu_id: queue.gpu_id(),
                desc: HeapDesc {
                    size: count * bytes_per_image,
                    properties: gpu::memory::CPU_VISIBLE,
                    resources: gpu::device::ResourceHeapType::Buffers,
                },
                result,
            };
            sender.send(msg).unwrap();
            let info = receiver.recv().unwrap();
            info.id
        };
        let gpu_heap_id = {
            let (result, receiver) = webgpu_channel().unwrap();
            let msg = WebGpuMsg::CreateHeap {
                gpu_id: queue.gpu_id(),
                desc: HeapDesc {
                    size: count * bytes_per_image,
                    properties: gpu::memory::DEVICE_LOCAL,
                    resources: gpu::device::ResourceHeapType::Images,
                },
                result,
            };
            sender.send(msg).unwrap();
            let info = receiver.recv().unwrap();
            info.id
        };
        let command_pool_info = {
            let (result, receiver) = webgpu_channel().unwrap();
            let msg = WebGpuMsg::CreateCommandPool {
                gpu_id: queue.gpu_id(),
                queue_id: queue.get_id(),
                flags: gpu::pool::RESET_INDIVIDUAL,
                result,
            };
            sender.send(msg).unwrap();
            receiver.recv().unwrap()
        };
        let command_buffer_receiver = {
            let (result, receiver) = webgpu_channel().unwrap();
            let command = WebGpuCommand::AllocateCommandBuffers(count as _, result);
            command_pool_info.channel.send(command).unwrap();
            receiver
        };

        let frames = (0..count).map(|i| {
            let image_id = {
                let (result, receiver) = webgpu_channel().unwrap();
                let msg = WebGpuMsg::CreateImage {
                    gpu_id: queue.gpu_id(),
                    desc: ImageDesc {
                        kind: gpu::image::Kind::D2(
                            size.width as gpu::image::Size,
                            size.height as gpu::image::Size,
                            gpu::image::AaMode::Single,
                        ),
                        levels: 1,
                        format: WebGpuDevice::map_format(format),
                        usage: gpu::image::COLOR_ATTACHMENT | gpu::image::TRANSFER_SRC,
                        heap_id: gpu_heap_id,
                        heap_offset: i * bytes_per_image,
                    },
                    result,
                };
                sender.send(msg).unwrap();
                let info = receiver.recv().unwrap();
                assert!(info.occupied_size <= bytes_per_image);
                info.id
            };
            let staging_buffer_id = {
                let (result, receiver) = webgpu_channel().unwrap();
                let msg = WebGpuMsg::CreateBuffer {
                    gpu_id: queue.gpu_id(),
                    desc: BufferDesc {
                        size: bytes_per_image,
                        stride: stride as _,
                        usage: gpu::buffer::TRANSFER_DST,
                        heap_id: staging_heap_id,
                        heap_offset: i * bytes_per_image,
                    },
                    result,
                };
                sender.send(msg).unwrap();
                let info = receiver.recv().unwrap();
                assert!(info.occupied_size <= bytes_per_image);
                info.id
            };
            let command_buffer_id = command_buffer_receiver.recv().unwrap().id;
            let fence_id = {
                let (result, receiver) = webgpu_channel().unwrap();
                let msg = WebGpuMsg::CreateFence {
                    gpu_id: queue.gpu_id(),
                    set: false,
                    result,
                };
                sender.send(msg).unwrap();
                receiver.recv().unwrap()
            };
            Frame {
                image: WebGpuImage::new(global, image_id),
                staging_buffer_id,
                command_buffer_id,
                fence_id,
            }
        }).collect();

        let obj = box WebGpuSwapchain {
            reflector_: Reflector::new(),
            sender,
            canvas_node: JS::from_ref(node),
            parent: WebGpuParent {
                context_id,
                gpu_id: queue.gpu_id(),
                queue_id: queue.get_id(),
            },
            command_pool_info,
            format,
            size,
            bytes_per_row,
            bytes_per_image,
            gpu_heap_id,
            staging_heap_id,
            frames,
            present_epoch: Cell::new(0),
            id_rotation: DOMRefCell::new(IdRotation::new(count as _)),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}

impl binding::WebGpuSwapchainMethods for WebGpuSwapchain {
    fn Format(&self) -> WebGpuFormat {
        self.format.clone()
    }

    fn GetSize(&self) -> WebGpuFramebufferSize {
        WebGpuFramebufferSize {
            width: self.size.width,
            height: self.size.height,
            layers: self.frames.len() as _,
        }
    }

    fn AcquireNextImage(&self,
        _semaphore: Option<&WebGpuSemaphore>,
    ) -> binding::WebGpuSwapchainImageId
    {
        //TODO: semaphore
        self.id_rotation.borrow_mut().acquire().unwrap()
    }

    fn GetImages(&self) -> Vec<Root<WebGpuImage>> {
        self.frames
            .iter()
            .map(|frame| frame.image.clone())
            .collect()
    }

    fn Present(&self) -> WebGpuImageState {
        let frame_id = self.id_rotation.borrow_mut().present().unwrap();
        let frame = &self.frames[frame_id as usize];
        let present_epoch = self.present_epoch.get();
        self.present_epoch.set(present_epoch + 1);

        let return_state = WebGpuImageState {
            access: gpu::image::TRANSFER_READ.bits() as _,
            layout: WebGpuImageLayout::TransferSrcOptimal,
        };
        let src_layout = gpu::image::ImageLayout::Present;
        let dst_layout = WebGpuDevice::map_image_layout(return_state.layout);
        let image_bar = ImageBarrier {
            state_src: (gpu::image::Access::empty(), src_layout),
            state_dst: (gpu::image::TRANSFER_READ, dst_layout),
            target: frame.image.get_id(),
        };
        let buffer_bar = BufferBarrier {
            state_src: gpu::buffer::TRANSFER_WRITE,
            state_dst: gpu::buffer::Access::empty(),
            target: frame.staging_buffer_id,
        };
        let region = gpu::command::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_pitch: self.bytes_per_row as _,
            buffer_slice_pitch: self.bytes_per_image as _,
            image_aspect: gpu::image::ASPECT_COLOR,
            image_subresource: (0, 0..1),
            image_offset: gpu::command::Offset {
                x: 0,
                y: 0,
                z: 0,
            },
            image_extent: gpu::command::Extent {
                width: self.size.width as _,
                height: self.size.height as _,
                depth: 1,
            },
        };

        let chan = &self.command_pool_info.channel;
        chan.send(WebGpuCommand::Begin(frame.command_buffer_id)).unwrap();
        chan.send(WebGpuCommand::PipelineBarrier {
            src_stages: gpu::pso::BOTTOM_OF_PIPE, //TODO
            dst_stages: gpu::pso::TRANSFER,
            buffer_bars: Vec::new(),
            image_bars: vec![image_bar],
        }).unwrap();
        chan.send(WebGpuCommand::CopyImageToBuffer {
            source_id: frame.image.get_id(),
            source_layout: dst_layout,
            destination_id: frame.staging_buffer_id,
            regions: vec![region],
        }).unwrap();
        chan.send(WebGpuCommand::PipelineBarrier {
            src_stages: gpu::pso::TRANSFER,
            dst_stages: gpu::pso::HOST,
            buffer_bars: vec![buffer_bar],
            image_bars: Vec::new(),
        }).unwrap();
        chan.send(WebGpuCommand::Finish(present_epoch)).unwrap();

        let msg_submit = WebGpuMsg::Submit {
            gpu_id: self.parent.gpu_id,
            queue_id: self.parent.queue_id,
            command_buffers: vec![SubmitInfo {
                pool_id: self.command_pool_info.id,
                cb_id: frame.command_buffer_id,
                submit_epoch: present_epoch,
            }],
            wait_semaphores: Vec::new(),
            signal_semaphores: Vec::new(),
            fence_id: Some(frame.fence_id),
        };
        self.sender.send(msg_submit).unwrap();

        let msg_present = WebGpuMsg::Present {
            context_id: self.parent.context_id,
            gpu_id: self.parent.gpu_id,
            buffer_id: frame.staging_buffer_id,
            bytes_per_row: self.bytes_per_row,
            fence_id: frame.fence_id,
            size: self.size,
        };
        self.sender.send(msg_present).unwrap();
        self.canvas_node.dirty(NodeDamage::OtherNodeDamage);

        return_state
    }
}
