/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::cell::Cell;

use canvas_traits::webgpu::{
    WebGpuChan, WebGpuCommand, WebGpuMsg, WebGpuReceiver,
    hal, webgpu_channel,
    BufferId, BufferDesc, CommandBufferId, CommandPoolInfo,
    FenceId, GpuId, MemoryId, MemoryDesc, BufferBarrier, ImageBarrier,
    ImageDesc, SubmitEpoch, SubmitInfo, QueueId,
    Presenter, PresentDone, ReadyFrame, WebGpuPresent,
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


#[derive(HeapSizeOf)]
pub struct Frame {
    image: Root<WebGpuImage>,
    staging_buffer_id: BufferId,
    command_buffer_id: CommandBufferId,
    fence_id: FenceId,
    #[ignore_heap_size_of = "Channels are hard"]
    presenting: DOMRefCell<Option<WebGpuReceiver<PresentDone>>>,
}

// dummy wrapper in order to avoid `dom_struct` errors
#[derive(HeapSizeOf)]
pub struct WebGpuParent {
    presenter: Presenter,
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
    gpu_mem_id: MemoryId,
    staging_mem_id: MemoryId,
    frames: Vec<Frame>,
    present_epoch: Cell<SubmitEpoch>, //TODO: atomics
    active_frame_id: Cell<u8>,
}

impl WebGpuSwapchain {
    pub fn compute_strides(size: Size2D<u32>, bytes_pp: usize, limits: &hal::Limits) -> (usize, usize) {
        let align = |x: usize, to: usize| { (x + to - 1) & !(to - 1) };
        let per_row = align(size.width as usize * bytes_pp, limits.min_buffer_copy_pitch_alignment);
        let per_image = align(align(size.height as usize, 0x100) * per_row, 0x10000);
        (per_row, per_image)
    }

    pub fn new(
        global: &GlobalScope,
        sender: WebGpuChan,
        node: &Node,
        presenter: Presenter,
        queue: &WebGpuCommandQueue,
        count: usize,
        format: WebGpuFormat,
        size: Size2D<u32>,
    ) -> Root<Self>
    {
        //Note: this constructor is an exception from the rule
        // as it's making the send calls internally, as opposed to
        // receiving an `*Info` struct having all the inputs

        presenter.send(WebGpuPresent::Enter(queue.gpu_id()));

        // rough upper bound for the image size
        let bytes_pp = 4usize;
        let (bytes_per_row, bytes_per_image) =
            Self::compute_strides(size, bytes_pp, queue.get_limits());

        let staging_mem_id = {
            let (result, receiver) = webgpu_channel().unwrap();
            let msg = WebGpuMsg::AllocateMemory {
                gpu_id: queue.gpu_id(),
                desc: MemoryDesc {
                    size: count * bytes_per_image,
                    ty: queue.find_heap_type(hal::memory::CPU_VISIBLE).unwrap(),
                },
                result,
            };
            sender.send(msg).unwrap();
            let info = receiver.recv().unwrap();
            info.id
        };
        let gpu_mem_id = {
            let (result, receiver) = webgpu_channel().unwrap();
            let msg = WebGpuMsg::AllocateMemory {
                gpu_id: queue.gpu_id(),
                desc: MemoryDesc {
                    size: count * bytes_per_image,
                    ty: queue.find_heap_type(hal::memory::DEVICE_LOCAL).unwrap(),
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
                flags: hal::pool::RESET_INDIVIDUAL,
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
            let image_info = {
                let (result, receiver) = webgpu_channel().unwrap();
                let msg = WebGpuMsg::CreateImage {
                    gpu_id: queue.gpu_id(),
                    desc: ImageDesc {
                        kind: hal::image::Kind::D2(
                            size.width as _,
                            size.height as _,
                            hal::image::AaMode::Single,
                        ),
                        levels: 1,
                        format: WebGpuDevice::map_format(format),
                        usage: hal::image::COLOR_ATTACHMENT | hal::image::TRANSFER_SRC,
                        mem_id: gpu_mem_id,
                        mem_offset: i * bytes_per_image,
                    },
                    result,
                };
                sender.send(msg).unwrap();
                let info = receiver.recv().unwrap();
                assert!(info.occupied_size <= bytes_per_image);
                info
            };
            let staging_buffer_id = {
                let (result, receiver) = webgpu_channel().unwrap();
                let msg = WebGpuMsg::CreateBuffer {
                    gpu_id: queue.gpu_id(),
                    desc: BufferDesc {
                        size: bytes_per_image,
                        stride: bytes_pp as _,
                        usage: hal::buffer::TRANSFER_DST,
                        mem_id: staging_mem_id,
                        mem_offset: i * bytes_per_image,
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
                image: WebGpuImage::new(global, image_info),
                staging_buffer_id,
                command_buffer_id,
                fence_id,
                presenting: DOMRefCell::new(None),
            }
        }).collect();

        let obj = box WebGpuSwapchain {
            reflector_: Reflector::new(),
            sender,
            canvas_node: JS::from_ref(node),
            parent: WebGpuParent {
                presenter,
                gpu_id: queue.gpu_id(),
                queue_id: queue.get_id(),
            },
            command_pool_info,
            format,
            size,
            bytes_per_row,
            bytes_per_image,
            gpu_mem_id,
            staging_mem_id,
            frames,
            present_epoch: Cell::new(0),
            active_frame_id: Cell::new(0),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }

    fn mark_dirty(&self) {
        self.canvas_node.dirty(NodeDamage::OtherNodeDamage);
    }
}

impl Drop for WebGpuSwapchain {
    fn drop(&mut self) {
        self.parent.presenter.send(WebGpuPresent::Exit);
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
        let mut id = self.active_frame_id.get() as usize + 1;
        if id == self.frames.len() {
            id = 0;
        }
        //println!("acquired {}", id);
        self.active_frame_id.set(id as _);

        if let Some(wait) = self.frames[id].presenting.borrow_mut().take() {
            //println!("waiting for frame with buffer {:?} ...", self.frames[id].staging_buffer_id);
            let _shown = wait.recv().unwrap();
            //println!("success: {}", shown);
        }
        id as _
    }

    fn GetImages(&self) -> Vec<Root<WebGpuImage>> {
        self.frames
            .iter()
            .map(|frame| frame.image.clone())
            .collect()
    }

    fn Present(&self) -> WebGpuImageState {
        let frame = &self.frames[self.active_frame_id.get() as usize];
        //println!("presenting {} with buffer id {:?}",
        //    self.active_frame_id.get(), frame.staging_buffer_id);
        let present_epoch = self.present_epoch.get();
        self.present_epoch.set(present_epoch + 1);

        let return_state = WebGpuImageState {
            access: hal::image::TRANSFER_READ.bits() as _,
            layout: WebGpuImageLayout::TransferSrcOptimal,
        };
        let src_layout = hal::image::ImageLayout::Present;
        let dst_layout = WebGpuDevice::map_image_layout(return_state.layout);
        let image_bar = ImageBarrier {
            states: (hal::image::Access::empty(), src_layout) .. (hal::image::TRANSFER_READ, dst_layout),
            target: frame.image.get_id(),
        };
        let buffer_bar = BufferBarrier {
            states: hal::buffer::TRANSFER_WRITE .. hal::buffer::Access::empty(),
            target: frame.staging_buffer_id,
        };
        let region = hal::command::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_pitch: self.bytes_per_row as _,
            buffer_slice_pitch: self.bytes_per_image as _,
            image_layers: hal::image::SubresourceLayers {
                aspects: hal::image::ASPECT_COLOR,
                level: 0,
                layers: 0 .. 1,
            },
            image_offset: hal::command::Offset {
                x: 0,
                y: 0,
                z: 0,
            },
            image_extent: hal::device::Extent {
                width: self.size.width as _,
                height: self.size.height as _,
                depth: 1,
            },
        };

        let chan = &self.command_pool_info.channel;
        chan.send(WebGpuCommand::Begin(frame.command_buffer_id)).unwrap();
        chan.send(WebGpuCommand::PipelineBarrier {
            stages: hal::pso::BOTTOM_OF_PIPE .. hal::pso::TRANSFER, //TODO?
            buffer_bars: Vec::new(),
            image_bars: vec![image_bar],
        }).unwrap();
        chan.send(WebGpuCommand::CopyImageToBuffer {
            source_id: frame.image.get_id(),
            source_layout: dst_layout,
            dest_id: frame.staging_buffer_id,
            regions: vec![region],
        }).unwrap();
        chan.send(WebGpuCommand::PipelineBarrier {
            stages: hal::pso::TRANSFER .. hal::pso::HOST,
            buffer_bars: vec![buffer_bar],
            image_bars: Vec::new(),
        }).unwrap();
        chan.send(WebGpuCommand::Finish(present_epoch)).unwrap();

        let (feedback_sender, feedback_receiver) = webgpu_channel().unwrap();
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
            feedback: Some(feedback_sender),
        };
        self.sender.send(msg_submit).unwrap();

        let (sender, receiver) = webgpu_channel().unwrap();
        debug_assert!(frame.presenting.borrow().is_none());
        *frame.presenting.borrow_mut() = Some(receiver);

        let ready_frame = ReadyFrame {
            buffer_id: frame.staging_buffer_id,
            bytes_per_row: self.bytes_per_row,
            fence_id: frame.fence_id,
            size: self.size,
            done_event: Some(sender),
            wait_event: Some(feedback_receiver),
        };

        self.parent.presenter.send(WebGpuPresent::Show(ready_frame));
        self.mark_dirty();

        return_state
    }
}
