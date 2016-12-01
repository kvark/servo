use glsl_to_spirv;
use shared_library::dynamic_library::DynamicLibrary;
use std::{iter, mem, ptr};
use std::io::Read;
use std::ffi::{CStr, CString};
use std::os::raw;
use std::path::Path;
use std::sync::Arc;
use vk;
use {CommandBuffer, CommandPool, Dimensions, Fence,
     FrameBuffer, FrameClearData, Queue,
     RenderPass, Pipeline, PipelineDesc, PipelineLayout,
     Shader, ShaderType, Share, SwapChain,
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

extern "system" fn callback(_ty: vk::DebugReportFlagsEXT,
                            _: vk::DebugReportObjectTypeEXT, _object: u64,
                            _location: usize, _msg_code: i32,
                            layer_prefix: *const raw::c_char,
                            description: *const raw::c_char,
                            _user_data: *mut raw::c_void)
                            -> u32 {
    unsafe {
        let layer_prefix = CStr::from_ptr(layer_prefix).to_str().unwrap();
        let description = CStr::from_ptr(description).to_str().unwrap();
        warn!("[{}] {}", layer_prefix, description);
        vk::FALSE
    }
}

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

        if debug {
            let info = vk::DebugReportCallbackCreateInfoEXT {
                sType: vk::STRUCTURE_TYPE_DEBUG_REPORT_CREATE_INFO_EXT,
                pNext: ptr::null(),
                flags: vk::DEBUG_REPORT_WARNING_BIT_EXT |
                       vk::DEBUG_REPORT_PERFORMANCE_WARNING_BIT_EXT |
                       vk::DEBUG_REPORT_ERROR_BIT_EXT,
                pfnCallback: callback,
                pUserData: ptr::null_mut(),
            };
            let mut output = 0;
            assert_eq!(vk::SUCCESS, unsafe {
                inst_pointers.CreateDebugReportCallbackEXT(instance, &info,
                                                           ptr::null(), &mut output)
            });
        }

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
        let queue = device.get_queue(qf_id as u32);

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

    fn get_queue(&self, family_id: u32) -> Queue {
        let mut out = 0;
        unsafe {
            self.share.vk.GetDeviceQueue(self.inner, family_id, 0, &mut out);
        };
        Queue::new(out, family_id)
    }

    pub fn make_command_pool(&self, family_id: u32) -> CommandPool {
        let info = vk::CommandPoolCreateInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
            pNext: ptr::null(),
            flags: vk::COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
            queueFamilyIndex: family_id,
        };

        let mut out = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.CreateCommandPool(self.inner, &info, ptr::null(), &mut out)
        });
        CommandPool::new(out, family_id)
    }

    pub fn make_command_buffer(&self, pool: &CommandPool) -> CommandBuffer {
        let info = vk::CommandBufferAllocateInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
            pNext: ptr::null(),
            commandPool: pool.get_inner(),
            level: vk::COMMAND_BUFFER_LEVEL_PRIMARY,
            commandBufferCount: 1,
        };

        let mut out = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.AllocateCommandBuffers(self.inner, &info, &mut out)
        });

        let fence = self.make_fence(true);
        CommandBuffer::new(out, pool.get_family_id(), fence)
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

    pub fn make_render_pass(&self, targets: &TargetSet)
                            -> (RenderPass, FrameClearData) {
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

        let mut clear_data = FrameClearData {
            colors: [[0.0; 4]; 4],
            depth: 0.0,
            stencil: 0,
        };
        let mut attachments = Vec::new();
        for (color, init) in targets.colors.iter().zip(clear_data.colors.iter_mut()) {
            let op = match color.1 {
                Some(v) => {
                    *init = v;
                    vk::ATTACHMENT_LOAD_OP_CLEAR
                },
                None => vk::ATTACHMENT_LOAD_OP_LOAD,
            };

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
            let depth_op = match ds.1 {
                Some(v) => {
                    clear_data.depth = v;
                    vk::ATTACHMENT_LOAD_OP_CLEAR
                },
                None => vk::ATTACHMENT_LOAD_OP_LOAD,
            };
            let stencil_op = match ds.2 {
                Some(v) => {
                    clear_data.stencil = v;
                    vk::ATTACHMENT_LOAD_OP_CLEAR
                },
                None => vk::ATTACHMENT_LOAD_OP_LOAD,
            };

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
        (RenderPass::new(pass, targets.colors.len(), attachments.len()),
         clear_data)
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

    pub fn make_shader(&self, code: &str, stype: ShaderType) -> Shader {
        let g2s_type = match stype {
            ShaderType::Vertex => glsl_to_spirv::ShaderType::Vertex,
            ShaderType::Fragment => glsl_to_spirv::ShaderType::Fragment,
        };
        let mut spirv = glsl_to_spirv::compile(code, g2s_type).unwrap();
        let mut data = Vec::new();
        spirv.read_to_end(&mut data).unwrap();

        let info = vk::ShaderModuleCreateInfo {
            sType: vk::STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
            pNext: ptr::null(),
            flags: 0,
            codeSize: data.len(),
            pCode: data.as_ptr() as *const u32,
        };

        let mut shader = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.CreateShaderModule(self.inner, &info, ptr::null(), &mut shader)
        });
        Shader::new(shader)
    }

    fn make_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        let info = vk::DescriptorSetLayoutCreateInfo {
            sType: vk::STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            pNext: ptr::null(),
            flags: 0,
            bindingCount: 0,
            pBindings: ptr::null(),
        };

        let mut set_layout = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.CreateDescriptorSetLayout(self.inner, &info, ptr::null(), &mut set_layout)
        });
        set_layout
    }

    pub fn make_pipeline_layout(&self) -> PipelineLayout {
        let set_layout = self.make_descriptor_set_layout();

        let info = vk::PipelineLayoutCreateInfo {
            sType: vk::STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
            pNext: ptr::null(),
            flags: 0,
            setLayoutCount: 1,
            pSetLayouts: &set_layout,
            pushConstantRangeCount: 0,
            pPushConstantRanges: ptr::null(),
        };

        let mut layout = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.CreatePipelineLayout(self.inner, &info, ptr::null(), &mut layout)
        });
        PipelineLayout::new(layout, vec![set_layout])
    }

    pub fn make_pipeline(&self, desc: &PipelineDesc, layout: &PipelineLayout,
                         pass: &RenderPass) -> Pipeline {
        let entry_point = b"main\0";
        let shaders = [
            vk::PipelineShaderStageCreateInfo {
                sType: vk::STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                stage: vk::SHADER_STAGE_VERTEX_BIT,
                module: desc.fun_vertex.get_inner(),
                pName: entry_point.as_ptr() as *const _,
                pSpecializationInfo: ptr::null(),
            },
            vk::PipelineShaderStageCreateInfo {
                sType: vk::STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                stage: vk::SHADER_STAGE_FRAGMENT_BIT,
                module: desc.fun_fragment.get_inner(),
                pName: entry_point.as_ptr() as *const _,
                pSpecializationInfo: ptr::null(),
            },
        ];
        let vertex_bindings = []; //TODO
        let vertex_attributes = [];
        let dynamic_states = [
            vk::DYNAMIC_STATE_VIEWPORT,
            vk::DYNAMIC_STATE_SCISSOR,
            vk::DYNAMIC_STATE_BLEND_CONSTANTS,
            vk::DYNAMIC_STATE_STENCIL_REFERENCE,
        ];
        let blends = [
            vk::PipelineColorBlendAttachmentState {
                colorWriteMask: 0xF,
                .. unsafe { mem::zeroed() }
            },
            vk::PipelineColorBlendAttachmentState {
                colorWriteMask: 0xF,
                .. unsafe { mem::zeroed() }
            },
            vk::PipelineColorBlendAttachmentState {
                colorWriteMask: 0xF,
                .. unsafe { mem::zeroed() }
            },
            vk::PipelineColorBlendAttachmentState {
                colorWriteMask: 0xF,
                .. unsafe { mem::zeroed() }
            },
        ];

        let info = vk::GraphicsPipelineCreateInfo {
            sType: vk::STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
            pNext: ptr::null(),
            flags: 0,
            stageCount: shaders.len() as u32,
            pStages: shaders.as_ptr(),
            pVertexInputState: &vk::PipelineVertexInputStateCreateInfo {
                sType: vk::STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                vertexBindingDescriptionCount: vertex_bindings.len() as u32,
                pVertexBindingDescriptions: vertex_bindings.as_ptr(),
                vertexAttributeDescriptionCount: vertex_attributes.len() as u32,
                pVertexAttributeDescriptions: vertex_attributes.as_ptr(),
            },
            pInputAssemblyState: &vk::PipelineInputAssemblyStateCreateInfo {
                sType: vk::STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                topology: vk::PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
                primitiveRestartEnable: vk::FALSE,
            },
            pTessellationState: ptr::null(),
            pViewportState: &vk::PipelineViewportStateCreateInfo {
                sType: vk::STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                viewportCount: 1,
                pViewports: &vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                    minDepth: 0.0,
                    maxDepth: 1.0,
                },
                scissorCount: 0,
                pScissors: ptr::null(),
            },
            pRasterizationState: &vk::PipelineRasterizationStateCreateInfo {
                sType: vk::STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                depthClampEnable: vk::FALSE,
                rasterizerDiscardEnable: vk::FALSE,
                polygonMode: vk::POLYGON_MODE_FILL,
                cullMode: vk::CULL_MODE_NONE,
                frontFace: vk::FRONT_FACE_COUNTER_CLOCKWISE,
                depthBiasEnable: vk::FALSE,
                depthBiasConstantFactor: 0.0,
                depthBiasClamp: 1.0,
                depthBiasSlopeFactor: 0.0,
                lineWidth: 1.0,
            },
            pMultisampleState: &vk::PipelineMultisampleStateCreateInfo {
                sType: vk::STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                rasterizationSamples: vk::SAMPLE_COUNT_1_BIT,
                sampleShadingEnable: vk::FALSE,
                minSampleShading: 0.0,
                pSampleMask: ptr::null(),
                alphaToCoverageEnable: vk::FALSE,
                alphaToOneEnable: vk::FALSE,
            },
            pDepthStencilState: &vk::PipelineDepthStencilStateCreateInfo {
                sType: vk::STRUCTURE_TYPE_PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                depthTestEnable: vk::FALSE,
                depthWriteEnable: vk::FALSE,
                depthCompareOp: vk::COMPARE_OP_NEVER,
                depthBoundsTestEnable: vk::FALSE,
                stencilTestEnable: vk::FALSE,
                front: unsafe { mem::zeroed() },
                back: unsafe { mem::zeroed() },
                minDepthBounds: 0.0,
                maxDepthBounds: 1.0,
            },
            pColorBlendState: &vk::PipelineColorBlendStateCreateInfo {
                sType: vk::STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                logicOpEnable: vk::FALSE,
                logicOp: vk::LOGIC_OP_CLEAR,
                attachmentCount: pass.get_num_colors() as u32,
                pAttachments: blends.as_ptr(),
                blendConstants: [0.0; 4],
            },
            pDynamicState: &vk::PipelineDynamicStateCreateInfo {
                sType: vk::STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                dynamicStateCount: dynamic_states.len() as u32,
                pDynamicStates: dynamic_states.as_ptr(),
            },
            layout: layout.get_inner(),
            renderPass: pass.get_inner(),
            subpass: 0,
            basePipelineHandle: 0,
            basePipelineIndex: 0,
        };

        let mut out = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.CreateGraphicsPipelines(self.inner, 0, 1, &info, ptr::null(), &mut out)
        });
        Pipeline::new(out, info.layout)
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

    pub fn read_frame(&mut self, texture: &Texture, layer: u32) -> DeviceMapper {
        //TODO: check for VkPhysicalDeviceLimits::minMemoryMapAlignment
        let layer_size = texture.get_layer_size();
        let mut ptr = ptr::null_mut();
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.MapMemory(self.inner,
                                    texture.memory,
                                    (layer_size * layer) as u64,
                                    layer_size as u64, 0, &mut ptr)
        });
        DeviceMapper {
            pointer: ptr as *const _,
            size: layer_size,
            memory: texture.memory,
            device: self,
        }
    }
}
