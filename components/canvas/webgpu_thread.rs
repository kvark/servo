/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu as w;
use webgpu::backend;
use webgpu::gpu::{self,
    Adapter, Device, QueueFamily,
    RawCommandBuffer, RawCommandPool, RawCommandQueue,
};

use euclid::Size2D;

use std::thread;
use std::collections::hash_map::{Entry, HashMap};
use std::sync::mpsc;

use webgpu_mode::LazyVec;

/// WebGL Threading API entry point that lives in the constellation.
/// It allows to get a WebGLThread handle for each script pipeline.
pub use ::webgpu_mode::WebGpuThreads;


enum InternalCommand {
    ExitPool(w::CommandPoolId),
}
enum PoolCommand<B: gpu::Backend> {
    FinishBuffer(w::CommandBufferInfo, w::SubmitEpoch, B::SubmitInfo),
}

struct CommandPoolHandle<B: gpu::Backend> {
    _join: thread::JoinHandle<()>,
    submits: HashMap<w::CommandBufferId, (w::CommandBufferEpoch, w::SubmitEpoch, B::SubmitInfo)>,
    receiver: mpsc::Receiver<PoolCommand<B>>,

}

impl<B: gpu::Backend> CommandPoolHandle<B> {
    fn receive_message(&mut self) {
        match self.receiver.recv().unwrap() {
            PoolCommand::FinishBuffer(cb_info, submit_epoch, submit_info) => {
                let value = (cb_info.epoch, submit_epoch, submit_info);
                self.submits.insert(cb_info.id, value);
            }
        }
    }

    fn extract_submit(&mut self,
        cb: w::CommandBufferInfo,
        submit_epoch: w::SubmitEpoch,
    ) -> B::SubmitInfo
    {
        loop {
            if let Entry::Occupied(e) = self.submits.entry(cb.id) {
                if e.get().0 == cb.epoch && e.get().1 == submit_epoch {
                    let value = e.remove();
                    return value.2
                }
            }
            self.receive_message();
        }
    }
}

pub struct WebGpuThread<B: gpu::Backend> {
    /// Id generator for new WebGpuContexts.
    next_webgpu_id: usize,
    adapters: Vec<B::Adapter>,
    gpus: LazyVec<gpu::Gpu<B>>,
    heaps: LazyVec<B::Heap>,
    images: LazyVec<B::Image>,
    command_pools: LazyVec<CommandPoolHandle<B>>,
}

impl WebGpuThread<backend::Backend> {
    /// Creates a new `WebGpuThread` and returns a Sender to
    /// communicate with it.
    pub fn start() -> w::WebGpuSender<w::WebGpuMsg> {
        let (sender, receiver) = w::webgpu_channel::<w::WebGpuMsg>().unwrap();
        let result = sender.clone();
        thread::Builder::new().name("WebGpuThread".to_owned()).spawn(move || {
            let instance = backend::Instance::create("Servo", 1);
            let mut renderer: Self = WebGpuThread {
                next_webgpu_id: 0,
                adapters: instance.enumerate_adapters(),
                gpus: LazyVec::new(),
                heaps: LazyVec::new(),
                images: LazyVec::new(),
                command_pools: LazyVec::new(),
            };
            let webgpu_chan = sender;
            loop {
                let msg = receiver.recv().unwrap();
                let exit = renderer.handle_msg(msg, &webgpu_chan);
                if exit {
                    return;
                }
            }
        }).expect("Thread spawning failed");

        result
    }
}

impl<B: gpu::Backend> WebGpuThread<B> {
    /// Handles a generic WebGpuMsg message
    fn handle_msg(&mut self, msg: w::WebGpuMsg, webgpu_chan: &w::WebGpuChan) -> bool {
        match msg {
            w::WebGpuMsg::CreateContext(sender) => {
                let init = self
                    .create_webgpu_context()
                    .map(|(id, adapters)| w::ContextInfo {
                        sender: w::WebGpuMsgSender::new(id, webgpu_chan.clone()),
                        adapters,
                    });
                sender.send(init).unwrap();
            }
            w::WebGpuMsg::OpenAdapter { adapter_id, queue_families, result } => {
                let adapter = &mut self.adapters[adapter_id as usize];
                let all_families = adapter.get_queue_families();
                let families = queue_families
                    .iter()
                    .map(|&(id, count)| {
                        let (ref family, type_) = all_families[id as usize];
                        (family, type_, count as u32)
                    })
                    .collect::<Vec<_>>();
                let gpu = adapter.open(&families);
                let general_queues = (0 .. gpu.general_queues.len() as w::QueueId).collect();
                let info = w::GpuInfo {
                    id: self.gpus.push(gpu).1 as w::GpuId,
                    general: general_queues,
                };
                result.send(info).unwrap();
            }
            w::WebGpuMsg::BuildSwapchain { gpu_id, size, result } => {
                let swapchain = self.build_swapchain(gpu_id, size);
                result.send(swapchain).unwrap();
            }
            w::WebGpuMsg::CreateCommandPool { gpu_id, queue_id, max_buffers, result } => {
                let command_pool = self.create_command_pool(gpu_id, queue_id, max_buffers);
                result.send(command_pool).unwrap();
            }
            w::WebGpuMsg::Submit { gpu_id, queue_id, command_buffers, .. } => {
                let cmd_buffers = command_buffers
                    .into_iter()
                    .map(|info| {
                        self.command_pools[info.pool_id as usize]
                            .extract_submit(info.cb, info.submit_epoch)
                    })
                    .collect();
                self.submit(gpu_id, queue_id, cmd_buffers);
            }
            w::WebGpuMsg::Present(image_id) => {
                //TODO
            }
            w::WebGpuMsg::Exit => {
                return true;
            }
        }

        false
    }

    /// Creates a new WebGpuContext
    fn create_webgpu_context(&mut self
    ) -> Result<(w::WebGpuContextId, Vec<w::AdapterInfo>), String>
    {
        let adapters = self.adapters
            .iter()
            .enumerate()
            .map(|(aid, ad)| {
                let queue_families = ad
                    .get_queue_families()
                    .iter()
                    .enumerate()
                    .map(|(qid, &(ref family, ty))| {
                        w::QueueFamilyInfo {
                            ty,
                            count: family.num_queues() as u8,
                            original_id: qid as w::QueueFamilyId,
                        }
                    })
                    .collect();
                w::AdapterInfo {
                    info: ad.get_info().clone(),
                    queue_families,
                    original_id: aid as w::AdapterId,
                }
            })
            .collect();
        let id = w::WebGpuContextId(self.next_webgpu_id);
        self.next_webgpu_id += 1;
        Ok((id, adapters))
    }

    fn build_swapchain(&mut self, gpu_id: w::GpuId, size: Size2D<u32>) -> w::SwapchainInfo {
        let gpu = &mut self.gpus[gpu_id as usize];
        let image_store = &mut self.images;
        let num_frames = 3; //TODO
        let alignment = 0x3F;
        let bytes_per_image = 4 *
            (size.width as u64 | alignment + 1)*
            (size.height as u64| alignment + 1);

        let heap = {
            let heap_type = gpu.heap_types
                .iter()
                .find(|ht| ht.properties.contains(gpu::memory::DEVICE_LOCAL))
                .unwrap();
            gpu.device.create_heap(
                heap_type,
                gpu::device::ResourceHeapType::Images,
                num_frames * bytes_per_image,
            ).unwrap()
        };

        let images = (0 .. num_frames).map(|i| {
            let unbound_image = gpu.device.create_image(
                gpu::image::Kind::D2(
                    size.width as gpu::image::Size,
                    size.height as gpu::image::Size,
                    gpu::image::AaMode::Single,
                ),
                1,
                <gpu::format::Srgba8 as gpu::format::Formatted>::get_format(),
                gpu::image::COLOR_ATTACHMENT | gpu::image::TRANSFER_SRC,
            ).unwrap();
            let image = gpu.device.bind_image_memory(
                &heap,
                i * bytes_per_image,
                unbound_image,
            ).unwrap();
            image_store.push(image).1 as w::ImageId
        }).collect();

        w::SwapchainInfo {
            heap_id: self.heaps.push(heap).1 as w::HeapId,
            images,
        }
    }

    #[allow(unsafe_code)]
    fn create_command_pool(&mut self,
        gpu_id: w::GpuId,
        queue_id: w::QueueId,
        max_buffers: u32,
    ) -> w::CommandPoolInfo
    {
        let gpu = &mut self.gpus[gpu_id as usize];
        let queue = gpu.general_queues[queue_id as usize].as_raw();//TODO
        let pool = unsafe {
            B::CommandPool::from_queue(queue, max_buffers as usize)
        };

        let (channel, receiver) = w::webgpu_channel().unwrap();
        let (int_sender, int_receiver) = mpsc::channel();

        let join = thread::spawn(move|| {
            Self::run_command_thread(receiver, int_sender, pool)
        });
        let handle = CommandPoolHandle {
            _join: join,
            submits: HashMap::new(),
            receiver: int_receiver,
        };
        let id = self.command_pools.push(handle).1 as w::CommandPoolId;

        w::CommandPoolInfo {
            channel,
            id,
        }
    }

    #[allow(unsafe_code)]
    fn run_command_thread(
        receiver: w::WebGpuReceiver<w::WebGpuCommand>,
        channel: mpsc::Sender<PoolCommand<B>>,
        mut pool: B::CommandPool,
    ) {
        let mut com_buffers = LazyVec::new();

        while let Ok(com) = receiver.recv() {
            match com {
                w::WebGpuCommand::Reset => {
                    pool.reset();
                }
                w::WebGpuCommand::Exit => {
                    //TODO
                    //channel.send(PoolCommand::Destroy).unwrap();
                    return
                }
                w::WebGpuCommand::AcquireCommandBuffer(result) => {
                    let cb = unsafe {
                        pool.acquire_command_buffer()
                    };
                    let (epoch, id) = com_buffers.push(cb);
                    let info = w::CommandBufferInfo {
                        id: id as w::CommandBufferId,
                        epoch: epoch as w::CommandBufferEpoch,
                    };
                    result.send(info).unwrap();
                }
                w::WebGpuCommand::ReturnCommandBuffer(id) => {
                    //TODO: notify the gpu thread?
                    let cb = com_buffers.pop(id as usize).unwrap();
                    unsafe {
                        pool.return_command_buffer(cb)
                    };
                }
                w::WebGpuCommand::Finish(info, submit_epoch) => {
                    //TODO: check cb epoch
                    let submit = com_buffers[info.id as usize].finish();
                    let cmd = PoolCommand::FinishBuffer(info, submit_epoch,submit);
                    channel.send(cmd).unwrap();
                }
                w::WebGpuCommand::PipelineBarrier(_, _) => {
                    //TODO
                }
            }
        }
    }

    #[allow(unsafe_code)]
    fn submit(&mut self,
        gpu_id: w::GpuId,
        queue_id: w::QueueId,
        cmd_buffers: Vec<B::SubmitInfo>,
    ) {
        let gpu = &mut self.gpus[gpu_id as usize];
        let queue = gpu.general_queues[queue_id as usize].as_mut();
        let submission = gpu::RawSubmission {
            cmd_buffers: &cmd_buffers,
            wait_semaphores: &[],
            signal_semaphores: &[],
        };
        unsafe {
            queue.submit_raw(submission, None)
        };
    }
}
