use canvas_traits::hal::{self, Device};
use self::hal::command::RawCommandBuffer;
use self::hal::pool::RawCommandPool;

use euclid::Size2D;


#[derive(Clone, Copy, Debug)]
pub enum ShareMode {
    /// Fast: a shared texture_id is used in WebRender.
    SharedTexture,
    /// Slow: reading pixels to RAM and providing to WR as an external image
    Readback,
}

struct Frame<B: hal::Backend> {
    offset: u64,
    image: B::Image,
    fence: B::Fence,
    command_buffer: B::CommandBuffer,
}

pub struct Swapchain<B: hal::Backend> {
    size: Size2D<u32>,
    format: hal::format::Format,
    frames: Vec<Frame<B>>,
    is_first_lock: bool,
    is_reading: bool,
    id_to_read: usize,
    id_to_present: usize,
    id_to_acquire: usize,
    _gpu_memory: B::Memory,
    cpu_memory: Option<B::Memory>,
    read_buffer: Option<B::Buffer>,
    _command_pool: B::CommandPool,
}

fn align(address: u64, alignment: u64) -> u64 {
    (address + alignment - 1) & !(alignment - 1)
}

impl<B: hal::Backend> Swapchain<B> {
    pub fn new(
        share_mode: ShareMode,
        size: Size2D<u32>, num_frames: usize, format: hal::format::Format,
        device: &B::Device, queue_family: hal::queue::QueueFamilyId,
        memory_types: &[hal::MemoryType]
    ) -> Self {
        use self::hal::image as i;

        let mut total_size = 0;
        let mut type_mask = !0u64;
        let mut preframes = Vec::new();

        for _ in 0 .. num_frames {
            let unbound = device
                .create_image(
                    i::Kind::D2(size.width, size.height, 1, 1),
                    1,
                    format,
                    i::Tiling::Optimal,
                    i::Usage::TRANSFER_SRC | i::Usage::COLOR_ATTACHMENT,
                    i::StorageFlags::empty(),
                )
                .unwrap();
            let req = device.get_image_requirements(&unbound);
            type_mask &= req.type_mask;
            let offset = align(total_size, req.alignment);
            total_size = offset + req.size;
            preframes.push((unbound, offset));
        }

        let (cpu_memory, read_buffer) = match share_mode {
            ShareMode::SharedTexture => (None, None),
            ShareMode::Readback => {
                let unbound = device.create_buffer(
                    total_size,
                    hal::buffer::Usage::TRANSFER_DST,
                    )
                    .unwrap();
                let requirements = device.get_buffer_requirements(&unbound);
                let cpu_type = memory_types
                    .iter()
                    .enumerate()
                    .position(|(i, mt)|
                        ((1 << i) as u64 & requirements.type_mask != 0) &&
                        mt.properties.contains(hal::memory::Properties::CPU_VISIBLE)
                    )
                    .unwrap()
                    .into();
                let memory = device
                    .allocate_memory(cpu_type, requirements.size)
                    .unwrap();
                let buffer = device
                    .bind_buffer_memory(&memory, 0, unbound)
                    .unwrap();
                (Some(memory), Some(buffer))
            },
        };

        let gpu_type = memory_types
            .iter()
            .enumerate()
            .position(|(i, mt)|
                ((1 << i) as u64 & type_mask != 0) &&
                mt.properties.contains(hal::memory::Properties::DEVICE_LOCAL)
            )
            .unwrap()
            .into();
        let gpu_memory = device
            .allocate_memory(gpu_type, total_size)
            .unwrap();

        let mut command_pool = device
            .create_command_pool(queue_family, hal::pool::CommandPoolCreateFlags::RESET_INDIVIDUAL);
        let cmd_buffers = command_pool.allocate(num_frames, hal::command::RawLevel::Primary);

        let frames = preframes
            .into_iter()
            .zip(cmd_buffers)
            .map(|((unbound, offset), command_buffer)| {
                let image = device
                    .bind_image_memory(&gpu_memory, offset, unbound)
                    .unwrap();
                let fence = device.create_fence(false);
                Frame {
                    offset,
                    image,
                    fence,
                    command_buffer,
                }
            })
            .collect();

        Swapchain {
            size,
            format,
            frames,
            is_first_lock: true,
            is_reading: false,
            id_to_read: 0,
            id_to_present: 0,
            id_to_acquire: 0,
            _gpu_memory: gpu_memory,
            cpu_memory,
            read_buffer,
            _command_pool: command_pool,
        }
    }

    pub fn frame_size(&self) -> u64 {
        let bypp = self.format.base_format().0.desc().bits >> 3;
        self.size.width  as u64 * self.size.height as u64 * bypp as u64
    }

    pub fn acquire_frame(&mut self, device: &B::Device) -> usize {
        let id = self.id_to_acquire;
        self.id_to_acquire = if id + 1 >= self.frames.len() { 0 } else { id + 1 };
        //device.wait_for_fence(&self.frames[id].fence, !0);
        if self.id_to_acquire == self.id_to_read {
            assert_ne!(self.id_to_read, self.id_to_present, "Swapchain frame capacity exceeded!");
            device.wait_for_fence(&self.frames[self.id_to_read].fence, !0);
            self.id_to_read += 1;
            if self.id_to_read == self.frames.len() {
                self.id_to_read = 0;
            }
        }
        id
    }

    #[allow(unsafe_code)]
    pub fn present(&mut self) -> (hal::command::Submit<B, hal::Graphics, hal::command::OneShot, hal::command::Primary>, &B::Fence) {
        use self::hal::{command as com, image as i};
        use self::hal::format::Aspects;
        // TODO: transition the buffer?
        assert_ne!(self.id_to_present, self.id_to_acquire, "Swapchain frame capacity exceeded!");
        let id = self.id_to_present;
        self.id_to_present = if id + 1 >= self.frames.len() { 0 } else { id + 1 };
        let frame = &mut self.frames[id];
        frame.command_buffer.begin(
            com::CommandBufferFlags::ONE_TIME_SUBMIT,
            com::CommandBufferInheritanceInfo::default(),
        );
        let mut cmd_buf = unsafe {
            com::CommandBuffer::new(&mut frame.command_buffer)
        };
        cmd_buf.pipeline_barrier(
            hal::pso::PipelineStage::BOTTOM_OF_PIPE .. hal::pso::PipelineStage::TRANSFER,
            hal::memory::Dependencies::empty(),
            [
                hal::memory::Barrier::Image {
                    states:
                        (i::Access::COLOR_ATTACHMENT_WRITE, i::Layout::ColorAttachmentOptimal) ..
                        (i::Access::TRANSFER_READ, i::Layout::TransferSrcOptimal),
                    target: &frame.image,
                    range: i::SubresourceRange {
                        aspects: Aspects::COLOR,
                        levels: 0 .. 1,
                        layers: 0 .. 1,
                    },
                },
            ].iter(),
        );
        cmd_buf.copy_image_to_buffer(
            &frame.image,
            i::Layout::TransferSrcOptimal,
            self.read_buffer.as_ref().unwrap(),
            [
                com::BufferImageCopy {
                    buffer_offset: frame.offset,
                    buffer_width: self.size.width,
                    buffer_height: self.size.height,
                    image_layers: i::SubresourceLayers {
                        aspects: Aspects::COLOR,
                        level: 0,
                        layers: 0 .. 1,
                    },
                    image_offset: i::Offset::ZERO,
                    image_extent: i::Extent {
                        width: self.size.width,
                        height: self.size.height,
                        depth: 1,
                    },
                },
            ].iter(),
        );
        cmd_buf.pipeline_barrier(
            hal::pso::PipelineStage::TRANSFER .. hal::pso::PipelineStage::TOP_OF_PIPE,
            hal::memory::Dependencies::empty(),
            [
                hal::memory::Barrier::Image {
                    states:
                        (i::Access::TRANSFER_READ, i::Layout::TransferSrcOptimal) ..
                        (i::Access::COLOR_ATTACHMENT_WRITE, i::Layout::ColorAttachmentOptimal),
                    target: &frame.image,
                    range: i::SubresourceRange {
                        aspects: Aspects::COLOR,
                        levels: 0 .. 1,
                        layers: 0 .. 1,
                    },
                },
            ].iter(),
        );
        (cmd_buf.finish(), &frame.fence)
    }

    pub fn read(&mut self, device: &B::Device) -> Option<*mut u8> {
        assert!(!self.is_reading);
        if self.id_to_read == self.id_to_present {
            return None
        }
        if self.is_first_lock {
            self.is_first_lock = false;
            device.wait_for_fence(&self.frames[self.id_to_read].fence, !0);
        }
        loop {
            let next = if self.id_to_read + 1 == self.frames.len() { 0 } else { self.id_to_read + 1 };
            if next == self.id_to_present {
                break
            }
            if !device.wait_for_fence(&self.frames[next].fence, 0) {
                break
            }
            self.id_to_read = next;
        };
        let offset = self.frames[self.id_to_read].offset;
        self.is_reading = true;
        match device.map_memory(
            self.cpu_memory.as_ref().unwrap(),
            offset .. offset + self.frame_size(),
        ) {
            Ok(ptr) => Some(ptr),
            Err(e) => {
                error!("Failed to map swapchain read buffer: {:?}", e);
                None
            }
        }
    }

    pub fn read_done(&mut self, device: &B::Device) {
        assert!(self.is_reading);
        self.is_reading = false;
        device.unmap_memory(self.cpu_memory.as_ref().unwrap());
    }
}
