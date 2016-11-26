use shared_library::dynamic_library::DynamicLibrary;
use std::{iter, mem, ptr};
use std::ffi::{CStr, CString};
use std::path::Path;
use std::sync::Arc;
use vk;
use {CommandBuffer, Dimensions, Fence, FrameBuffer, Queue,
     RenderPass, Share, SwapChain,
     TargetSet, TargetView, Texture, WebMetalCapabilities};

const LAYERS: &'static [&'static str] = &[
];
const LAYERS_DEBUG: &'static [&'static str] = &[
    "VK_LAYER_LUNARG_standard_validation",
];
const EXTENSIONS: &'static [&'static str] = &[
    "VK_KHR_surface",
];
const EXTENSIONS_DEBUG: &'static [&'static str] = &[
    "VK_KHR_surface",
    "VK_EXT_debug_report",
];
const DEV_EXTENSIONS: &'static [&'static str] = &[
    "VK_KHR_swapchain",
];
const SURFACE_EXTENSIONS: &'static [&'static str] = &[
    // Platform-specific WSI extensions
    "VK_KHR_xlib_surface",
    "VK_KHR_xcb_surface",
    "VK_KHR_wayland_surface",
    "VK_KHR_mir_surface",
    "VK_KHR_android_surface",
    "VK_KHR_win32_surface",
];


struct PhysicalDeviceInfo {
    device: vk::PhysicalDevice,
    _properties: vk::PhysicalDeviceProperties,
    queue_families: Vec<vk::QueueFamilyProperties>,
    memory: vk::PhysicalDeviceMemoryProperties,
    _features: vk::PhysicalDeviceFeatures,
}

impl PhysicalDeviceInfo {
    #[allow(unsafe_code)]
    pub fn new(dev: vk::PhysicalDevice, vk: &vk::InstancePointers) -> PhysicalDeviceInfo {
        PhysicalDeviceInfo {
            device: dev,
            _properties: unsafe {
                let mut out = mem::zeroed();
                vk.GetPhysicalDeviceProperties(dev, &mut out);
                out
            },
            queue_families: unsafe {
                let mut num = 0;
                vk.GetPhysicalDeviceQueueFamilyProperties(dev, &mut num, ptr::null_mut());
                let mut families = Vec::with_capacity(num as usize);
                vk.GetPhysicalDeviceQueueFamilyProperties(dev, &mut num, families.as_mut_ptr());
                families.set_len(num as usize);
                families
            },
            memory: unsafe {
                let mut out = mem::zeroed();
                vk.GetPhysicalDeviceMemoryProperties(dev, &mut out);
                out
            },
            _features: unsafe {
                let mut out = mem::zeroed();
                vk.GetPhysicalDeviceFeatures(dev, &mut out);
                out
            },
        }
    }
}


pub struct DeviceMapper<'a> {
    pub pointer: *const u8,
    pub size: u32,
    memory: vk::DeviceMemory,
    device: &'a Device,
}

impl<'a> Drop for DeviceMapper<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.share.vk.UnmapMemory(self.device.inner, self.memory);
        }
    }
}

pub struct Device {
    _dyn_lib: DynamicLibrary,
    _library: vk::Static,
    inner: vk::Device,
    pub share: Arc<Share>,
    mem_system: u32,
    mem_video: u32,
}

#[derive(Debug)]
pub struct InitError;

impl Device {
    pub fn new(debug: bool)
               -> Result<(Device, Queue, WebMetalCapabilities), InitError> {
        let (layers, extensions) = if debug {
            (LAYERS_DEBUG, EXTENSIONS_DEBUG)
        } else {
            (LAYERS, EXTENSIONS)
        };
        let dev_extensions = DEV_EXTENSIONS;

        let lib_name = if cfg!(target_os = "windows") {
            "vulkan-1.dll"
        } else {
            "libvulkan.so.1"
        };
        let dynamic_lib = DynamicLibrary::open(Some(Path::new(lib_name)))
                                         .expect("Unable to open vulkan shared library");

        let lib = vk::Static::load(|name| unsafe {
            let name = name.to_str().unwrap();
            dynamic_lib.symbol(name).unwrap()
        });
        let entry_points = vk::EntryPoints::load(|name| unsafe {
            mem::transmute(lib.GetInstanceProcAddr(0, name.as_ptr()))
        });

        let app_info = vk::ApplicationInfo {
            sType: vk::STRUCTURE_TYPE_APPLICATION_INFO,
            pNext: ptr::null(),
            pApplicationName: "servo".as_ptr() as *const _,
            applicationVersion: 1,
            pEngineName: "webmetal".as_ptr() as *const _,
            engineVersion: 0x1000, //TODO
            apiVersion: 0x400000, //TODO
        };

        let instance_extensions = {
            let mut num = 0;
            assert_eq!(vk::SUCCESS, unsafe {
                entry_points.EnumerateInstanceExtensionProperties(ptr::null(), &mut num, ptr::null_mut())
            });
            let mut out = Vec::with_capacity(num as usize);
            assert_eq!(vk::SUCCESS, unsafe {
                entry_points.EnumerateInstanceExtensionProperties(ptr::null(), &mut num, out.as_mut_ptr())
            });
            unsafe { out.set_len(num as usize); }
            out
        };

        // Check our surface extensions against the available extensions
        let surface_extensions = SURFACE_EXTENSIONS.iter().filter_map(|ext| {
            instance_extensions.iter().find(|inst_ext| {
                unsafe { CStr::from_ptr(inst_ext.extensionName.as_ptr()) == CStr::from_ptr(ext.as_ptr() as *const i8) }
            }).and_then(|_| Some(*ext))
        }).collect::<Vec<&str>>();

        let instance = {
            let cstrings = layers.iter().chain(extensions.iter())
                                        .chain(surface_extensions.iter())
                             .map(|&s| CString::new(s).unwrap())
                             .collect::<Vec<_>>();
            let str_pointers = cstrings.iter()
                                       .map(|s| s.as_ptr())
                                       .collect::<Vec<_>>();

            let create_info = vk::InstanceCreateInfo {
                sType: vk::STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                pApplicationInfo: &app_info,
                enabledLayerCount: layers.len() as u32,
                ppEnabledLayerNames: str_pointers.as_ptr(),
                enabledExtensionCount: (extensions.len() + surface_extensions.len()) as u32,
                ppEnabledExtensionNames: str_pointers[layers.len()..].as_ptr(),
            };
            let mut out = 0;
            assert_eq!(vk::SUCCESS, unsafe {
                entry_points.CreateInstance(&create_info, ptr::null(), &mut out)
            });
            out
        };

        let inst_pointers = vk::InstancePointers::load(|name| unsafe {
            mem::transmute(lib.GetInstanceProcAddr(instance, name.as_ptr()))
        });

        let physical_devices = {
            let mut num = 0;
            assert_eq!(vk::SUCCESS, unsafe {
                inst_pointers.EnumeratePhysicalDevices(instance, &mut num, ptr::null_mut())
            });
            let mut devices = Vec::with_capacity(num as usize);
            assert_eq!(vk::SUCCESS, unsafe {
                inst_pointers.EnumeratePhysicalDevices(instance, &mut num, devices.as_mut_ptr())
            });
            unsafe { devices.set_len(num as usize); }
            devices
        };

        let phys_infos = physical_devices.iter()
            .map(|dev| PhysicalDeviceInfo::new(*dev, &inst_pointers))
            .collect::<Vec<_>>();

        let (dev, (qf_id, _))  = phys_infos.iter()
            .flat_map(|d| iter::repeat(d).zip(d.queue_families.iter().enumerate()))
            .find(|&(_, (_, qf))| qf.queueFlags & vk::QUEUE_GRAPHICS_BIT != 0)
            .unwrap();
        //info!("Chosen physical device {:?} with queue family {}", dev.device, qf_id);

        let mvid_id = dev.memory.memoryTypes.iter().take(dev.memory.memoryTypeCount as usize)
                                .position(|mt| mt.propertyFlags & vk::MEMORY_PROPERTY_DEVICE_LOCAL_BIT != 0)
                                .unwrap() as u32;
        let msys_id = dev.memory.memoryTypes.iter().take(dev.memory.memoryTypeCount as usize)
                                .position(|mt| (mt.propertyFlags & vk::MEMORY_PROPERTY_HOST_COHERENT_BIT != 0)
                                            && (mt.propertyFlags & vk::MEMORY_PROPERTY_HOST_VISIBLE_BIT != 0))
                                .unwrap() as u32;

        let vk_device = {
            let cstrings = dev_extensions.iter()
                                         .map(|&s| CString::new(s).unwrap())
                                         .collect::<Vec<_>>();
            let str_pointers = cstrings.iter().map(|s| s.as_ptr())
                                       .collect::<Vec<_>>();

            let queue_info = vk::DeviceQueueCreateInfo {
                sType: vk::STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                queueFamilyIndex: qf_id as u32,
                queueCount: 1,
                pQueuePriorities: &1.0,
            };
            let features = unsafe{ mem::zeroed() };

            let dev_info = vk::DeviceCreateInfo {
                sType: vk::STRUCTURE_TYPE_DEVICE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                queueCreateInfoCount: 1,
                pQueueCreateInfos: &queue_info,
                enabledLayerCount: 0,
                ppEnabledLayerNames: ptr::null(),
                enabledExtensionCount: str_pointers.len() as u32,
                ppEnabledExtensionNames: str_pointers.as_ptr(),
                pEnabledFeatures: &features,
            };
            let mut out = 0;
            assert_eq!(vk::SUCCESS, unsafe {
                inst_pointers.CreateDevice(dev.device, &dev_info, ptr::null(), &mut out)
            });
            out
        };

        let dev_pointers = vk::DevicePointers::load(|name| unsafe {
            inst_pointers.GetDeviceProcAddr(vk_device, name.as_ptr()) as *const _
        });

        let device = Device {
            _dyn_lib: dynamic_lib,
            _library: lib,
            inner: vk_device,
            share: Arc::new(Share {
                vk: dev_pointers
            }),
            mem_system: msys_id,
            mem_video: mvid_id,
        };
        let queue = device.make_queue(qf_id as u32);

        Ok((device, queue, WebMetalCapabilities))
    }

    fn alloc(&self, mem_id: u32, reqs: vk::MemoryRequirements) -> vk::DeviceMemory {
        let info = vk::MemoryAllocateInfo {
            sType: vk::STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
            pNext: ptr::null(),
            allocationSize: reqs.size,
            memoryTypeIndex: mem_id,
        };
        let mut mem = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.AllocateMemory(self.inner, &info, ptr::null(), &mut mem)
        });
        mem
    }

    fn make_queue(&self, family_id: u32) -> Queue {
        let com_info = vk::CommandPoolCreateInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
            pNext: ptr::null(),
            flags: vk::COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
            queueFamilyIndex: family_id,
        };
        let mut com_pool = 0;
        let vk = &self.share.vk;
        assert_eq!(vk::SUCCESS, unsafe {
            vk.CreateCommandPool(self.inner, &com_info, ptr::null(), &mut com_pool)
        });

        let queue = unsafe {
            let mut out = mem::zeroed();
            vk.GetDeviceQueue(self.inner, family_id, 0, &mut out);
            out
        };
        Queue::new(queue, family_id, com_pool)
    }

    pub fn make_command_buffer(&self, queue: &Queue) -> CommandBuffer {
        let info = vk::CommandBufferAllocateInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
            pNext: ptr::null(),
            commandPool: queue.get_pool(),
            level: vk::COMMAND_BUFFER_LEVEL_PRIMARY,
            commandBufferCount: 1,
        };
        let mut buf = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.AllocateCommandBuffers(self.inner, &info, &mut buf)
        });
        CommandBuffer::new(buf, queue.get_family_id())
    }

    pub fn make_fence(&self, signalled: bool) -> Fence {
        let info = vk::FenceCreateInfo {
            sType: vk::STRUCTURE_TYPE_FENCE_CREATE_INFO,
            pNext: ptr::null(),
            flags: if signalled {vk::FENCE_CREATE_SIGNALED_BIT} else {0}
        };
        let mut fence = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.CreateFence(self.inner, &info, ptr::null(), &mut fence)
        });
        Fence::new(fence)
    }

    pub fn check_fence(&self, fence: &Fence) -> bool {
        let res = unsafe {
            self.share.vk.GetFenceStatus(self.inner, fence.get_inner())
        };
        if res == vk::NOT_READY {
            false
        } else {
            assert_eq!(res, vk::SUCCESS);
            true
        }
    }

    pub fn wait_fence(&self, fence: &Fence, timeout: u64) -> bool {
        let res = unsafe {
            self.share.vk.WaitForFences(self.inner, 1, &fence.get_inner(), vk::FALSE, timeout)
        };
        if res == vk::TIMEOUT {
            false
        } else {
            assert_eq!(res, vk::SUCCESS);
            true
        }
    }

    pub fn make_render_pass(&self, targets: &TargetSet) -> RenderPass {
        let color_references = [
            vk::AttachmentReference {
                attachment: 0,
                layout: vk::IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
            },
            vk::AttachmentReference {
                attachment: 1,
                layout: vk::IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
            },
            vk::AttachmentReference {
                attachment: 2,
                layout: vk::IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
            },
            vk::AttachmentReference {
                attachment: 3,
                layout: vk::IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
            },
        ];
        let depth_stencil_ref = vk::AttachmentReference {
            attachment: targets.colors.len() as u32,
            layout: vk::IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };

        let sub_pass = vk::SubpassDescription {
            flags: 0,
            pipelineBindPoint: vk::PIPELINE_BIND_POINT_GRAPHICS,
            inputAttachmentCount: 0,
            pInputAttachments: ptr::null(),
            colorAttachmentCount: targets.colors.len() as u32,
            pColorAttachments: color_references.as_ptr(),
            pResolveAttachments: ptr::null(),
            pDepthStencilAttachment: if targets.depth_stencil.is_some() {
                &depth_stencil_ref
            } else {
                ptr::null()
            },
            preserveAttachmentCount: 0,
            pPreserveAttachments: ptr::null(),
        };

        let mut clears = Vec::new();
        let mut attachments = Vec::new();
        for color in targets.colors.iter() {
            let (op, clear) = match color.1 {
                Some(v) => (vk::ATTACHMENT_LOAD_OP_CLEAR, v),
                None => (vk::ATTACHMENT_LOAD_OP_LOAD, [0.0; 4]),
            };
            clears.push(vk::ClearValue::color(vk::ClearColorValue::float32(clear)));

            attachments.push(vk::AttachmentDescription {
                flags: 0,
                format: color.0.texture.format,
                samples: color.0.texture.samples,
                loadOp: op,
                storeOp: vk::ATTACHMENT_STORE_OP_STORE,
                stencilLoadOp: vk::ATTACHMENT_LOAD_OP_LOAD,
                stencilStoreOp: vk::ATTACHMENT_STORE_OP_STORE,
                initialLayout: color.0.texture.default_layout,
                finalLayout: color.0.texture.default_layout,
            })
        }
        if let Some(ref ds) = targets.depth_stencil {
            let (depth_op, depth_clear) = match ds.1 {
                Some(v) => (vk::ATTACHMENT_LOAD_OP_CLEAR, v),
                None => (vk::ATTACHMENT_LOAD_OP_LOAD, 0.0)
            };
            let (stencil_op, stencil_clear) = match ds.2 {
                Some(v) => (vk::ATTACHMENT_LOAD_OP_CLEAR, v),
                None => (vk::ATTACHMENT_LOAD_OP_LOAD, 0)
            };
            clears.push(vk::ClearValue::depth_stencil(vk::ClearDepthStencilValue {
                depth: depth_clear,
                stencil: stencil_clear as u32,
            }));

            attachments.push(vk::AttachmentDescription {
                flags: 0, //vk::ATTACHMENT_DESCRIPTION_MAY_ALIAS_BIT,
                format: ds.0.texture.format,
                samples: ds.0.texture.samples,
                loadOp: depth_op,
                storeOp: vk::ATTACHMENT_STORE_OP_STORE,
                stencilLoadOp: stencil_op,
                stencilStoreOp: vk::ATTACHMENT_STORE_OP_STORE,
                initialLayout: ds.0.texture.default_layout,
                finalLayout: ds.0.texture.default_layout,
            })
        }

        let info = vk::RenderPassCreateInfo {
            sType: vk::STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
            pNext: ptr::null(),
            flags: 0,
            attachmentCount: attachments.len() as u32,
            pAttachments: attachments.as_ptr(),
            subpassCount: 1,
            pSubpasses: &sub_pass,
            dependencyCount: 0,
            pDependencies: ptr::null(),
        };

        let mut pass = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.CreateRenderPass(self.inner, &info, ptr::null(), &mut pass)
        });
        RenderPass::new(pass, clears)
    }

    pub fn make_frame_buffer(&self, targets: &TargetSet, pass: &RenderPass)
                             -> FrameBuffer {
        let mut attachments = Vec::new();
        let mut dim = Dimensions { w: 0, h: 0, d: 0 };
        for color in targets.colors.iter() {
            attachments.push(color.0.inner);
            if dim.w == 0 {
                dim = color.0.texture.dim.clone();
            } else {
                assert_eq!(dim, color.0.texture.dim);
            };
        }
        if let Some(ref ds) = targets.depth_stencil {
            attachments.push(ds.0.inner);
            if dim.w == 0 {
                dim = ds.0.texture.dim.clone();
            } else {
                assert_eq!(dim, ds.0.texture.dim);
            };
        }

        let info = vk::FramebufferCreateInfo {
            sType: vk::STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
            pNext: ptr::null(),
            flags: 0,
            renderPass: pass.inner,
            attachmentCount: attachments.len() as u32,
            pAttachments: attachments.as_ptr(),
            width: dim.w,
            height: dim.h,
            layers: 1,
        };

        let mut fbuf = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.CreateFramebuffer(self.inner, &info, ptr::null(), &mut fbuf)
        });
        FrameBuffer::new(fbuf, dim)
    }

    pub fn make_swap_chain(&self, width: u32, height: u32,
                           gpu_frame_count: u32, cpu_frame_count: u32)
                           -> SwapChain {
        let vk = &self.share.vk;
        let gpu_texture = {
            let info = vk::ImageCreateInfo {
                sType: vk::STRUCTURE_TYPE_IMAGE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                imageType: vk::IMAGE_TYPE_2D,
                format: vk::FORMAT_R8G8B8A8_SRGB,
                extent: vk::Extent3D {
                    width: width,
                    height: height,
                    depth: 1,
                },
                mipLevels: 1,
                arrayLayers: gpu_frame_count,
                samples: vk::SAMPLE_COUNT_1_BIT,
                tiling: vk::IMAGE_TILING_OPTIMAL,
                usage: vk::IMAGE_USAGE_TRANSFER_SRC_BIT | vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
                sharingMode: vk::SHARING_MODE_EXCLUSIVE,
                queueFamilyIndexCount: 0,
                pQueueFamilyIndices: ptr::null(),
                initialLayout: vk::IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
            };

            let mut image = 0;
            assert_eq!(vk::SUCCESS, unsafe {
                vk.CreateImage(self.inner, &info, ptr::null(), &mut image)
            });
            let reqs = unsafe {
                let mut out = mem::zeroed();
                vk.GetImageMemoryRequirements(self.inner, image, &mut out);
                out
            };
            let memory = self.alloc(self.mem_video, reqs);
            assert_eq!(vk::SUCCESS, unsafe {
                vk.BindImageMemory(self.inner, image, memory, 0)
            });
            Arc::new(Texture {
                inner: image,
                memory: memory,
                default_layout: info.initialLayout,
                dim: info.extent.into(),
                usage: info.usage,
                format: info.format,
                samples: info.samples,
            })
        };
        let cpu_texture = {
            let info = vk::ImageCreateInfo {
                sType: vk::STRUCTURE_TYPE_IMAGE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                imageType: vk::IMAGE_TYPE_2D,
                format: vk::FORMAT_R8G8B8A8_UNORM,
                extent: vk::Extent3D {
                    width: width,
                    height: height,
                    depth: 1,
                },
                mipLevels: 1,
                arrayLayers: cpu_frame_count,
                samples: vk::SAMPLE_COUNT_1_BIT,
                tiling: vk::IMAGE_TILING_LINEAR,
                usage: vk::IMAGE_USAGE_TRANSFER_DST_BIT,
                sharingMode: vk::SHARING_MODE_EXCLUSIVE,
                queueFamilyIndexCount: 0,
                pQueueFamilyIndices: ptr::null(),
                initialLayout: vk::IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
            };

            let mut image = 0;
            assert_eq!(vk::SUCCESS, unsafe {
                vk.CreateImage(self.inner, &info, ptr::null(), &mut image)
            });
            let reqs = unsafe {
                let mut out = mem::zeroed();
                vk.GetImageMemoryRequirements(self.inner, image, &mut out);
                out
            };
            let memory = self.alloc(self.mem_system, reqs);
            assert_eq!(vk::SUCCESS, unsafe {
                vk.BindImageMemory(self.inner, image, memory, 0)
            });
            Arc::new(Texture {
                inner: image,
                memory: memory,
                default_layout: info.initialLayout,
                dim: info.extent.into(),
                usage: info.usage,
                format: info.format,
                samples: info.samples,
            })
        };
        let views = (0 .. gpu_frame_count).map(|i| {
            let info = vk::ImageViewCreateInfo {
                sType: vk::STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                image: gpu_texture.inner,
                viewType: vk::IMAGE_VIEW_TYPE_2D,
                format: vk::FORMAT_R8G8B8A8_SRGB,
                components: vk::ComponentMapping {
                    r: vk::COMPONENT_SWIZZLE_IDENTITY,
                    g: vk::COMPONENT_SWIZZLE_IDENTITY,
                    b: vk::COMPONENT_SWIZZLE_IDENTITY,
                    a: vk::COMPONENT_SWIZZLE_IDENTITY,
                },
                subresourceRange: vk::ImageSubresourceRange {
                    aspectMask: vk::IMAGE_ASPECT_COLOR_BIT,
                    baseMipLevel: 0,
                    levelCount: 1,
                    baseArrayLayer: i,
                    layerCount: 1,
                },
            };

            let mut view = 0;
            assert_eq!(vk::SUCCESS, unsafe {
                vk.CreateImageView(self.inner, &info, ptr::null(), &mut view)
            });
            TargetView {
                inner: view,
                layer: i,
                texture: gpu_texture.clone(),
            }
        }).collect();

        SwapChain {
            gpu_texture: gpu_texture,
            cpu_texture: cpu_texture,
            cpu_layer_count: cpu_frame_count,
            cpu_current_layer: 0,
            views: views,
        }
    }

    pub fn read_frame(&mut self, swap_chain: &SwapChain) -> DeviceMapper {
        //TODO: check for VkPhysicalDeviceLimits::minMemoryMapAlignment
        let layer_size = swap_chain.cpu_texture.get_layer_size();
        let mut ptr = ptr::null_mut();
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.MapMemory(self.inner,
                                    swap_chain.cpu_texture.memory,
                                    (layer_size * swap_chain.cpu_current_layer) as u64,
                                    layer_size as u64, 0, &mut ptr)
        });
        DeviceMapper {
            pointer: ptr as *const _,
            size: layer_size,
            memory: swap_chain.cpu_texture.memory,
            device: self,
        }
    }
}
