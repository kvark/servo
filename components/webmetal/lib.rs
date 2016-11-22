/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![crate_name = "webmetal"]
#![crate_type = "rlib"]
#![feature(plugin)]
#![feature(proc_macro)]
#![plugin(plugins)]

#[macro_use] extern crate serde_derive;
extern crate shared_library;
extern crate vk_sys as vk;

use shared_library::dynamic_library::DynamicLibrary;
use std::{iter, mem, ptr};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
pub struct WebMetalCapabilities;

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

pub struct Share {
    vk: vk::DevicePointers,
}

pub struct ResourceState {
    image_layouts: HashMap<(Arc<Texture>, u32), vk::ImageLayout>,
}

impl ResourceState {
    pub fn new() -> ResourceState {
        ResourceState {
            image_layouts: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommandBuffer {
    inner: vk::CommandBuffer,
    family_index: u32,
}

impl CommandBuffer {
    pub fn begin(&self, share: &Share) {
        let info = vk::CommandBufferBeginInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            pNext: ptr::null(),
            flags: 0,
            pInheritanceInfo: ptr::null(),
        };
        assert_eq!(vk::SUCCESS, unsafe {
            share.vk.BeginCommandBuffer(self.inner, &info)
        });
    }

    fn set_image_layout(&self, share: &Share, res: &mut ResourceState,
                        texture: &Arc<Texture>, layer: u32, layout: vk::ImageLayout) {
        let key = (texture.clone(), layer);
        let old_layout = *res.image_layouts.get(&key)
                                           .unwrap_or(&texture.default_layout);
        if layout != old_layout {
            self.image_barrier(share, &texture, layer, old_layout, layout);
            res.image_layouts.insert(key, layout);
        }
    }

    pub fn reset_state(&self, share: &Share, res: ResourceState) {
        for ((texture, layer), layout) in res.image_layouts.into_iter() {
            if layout != texture.default_layout {
                self.image_barrier(share, &texture, layer, layout, texture.default_layout)
            }
        }
    }

    fn image_barrier(&self, share: &Share, texture: &Texture, layer: u32,
                     old_layout: vk::ImageLayout, new_layout: vk::ImageLayout) {
        let mut access = 0; //TODO
        if texture.usage & vk::IMAGE_USAGE_TRANSFER_SRC_BIT != 0 {
            access |= vk::ACCESS_TRANSFER_WRITE_BIT;
        }
        if texture.usage & vk::IMAGE_USAGE_TRANSFER_DST_BIT != 0 {
            access |= vk::ACCESS_TRANSFER_READ_BIT;
        }
        if texture.usage & vk::IMAGE_USAGE_SAMPLED_BIT != 0 {
            access |= vk::ACCESS_SHADER_READ_BIT;
        }
        let barrier = vk::ImageMemoryBarrier {
            sType: vk::STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
            pNext: ptr::null(),
            srcAccessMask: access,
            dstAccessMask: access,
            oldLayout: old_layout,
            newLayout: new_layout,
            srcQueueFamilyIndex: self.family_index,
            dstQueueFamilyIndex: self.family_index,
            image: texture.inner,
            subresourceRange: vk::ImageSubresourceRange {
                aspectMask: vk::IMAGE_ASPECT_COLOR_BIT,
                baseMipLevel: 0,
                levelCount: 1,
                baseArrayLayer: layer,
                layerCount: 1,
            },
        };
        unsafe {
            share.vk.CmdPipelineBarrier(self.inner,
                vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, 0,
                0, ptr::null(), 0, ptr::null(), 1, &barrier);
        }
    }

    pub fn clear_color(&self, share: &Share, res: &mut ResourceState,
                       view: &TargetView, color: [f32; 4]) {
        let layout = vk::IMAGE_LAYOUT_GENERAL;
        self.set_image_layout(share, res, &view.texture, view.layer, layout);

        let value = vk::ClearColorValue::float32(color);
        let range = vk::ImageSubresourceRange {
            aspectMask: vk::IMAGE_ASPECT_COLOR_BIT,
            baseMipLevel: 0,
            levelCount: 1,
            baseArrayLayer: view.layer,
            layerCount: 1,
        };
        unsafe {
            share.vk.CmdClearColorImage(self.inner,
                                        view.texture.inner,
                                        layout,
                                        &value, 1, &range);
        }
    }

    pub fn copy_texture(&self, share: &Share, res: &mut ResourceState,
                        src: &Arc<Texture>, src_layer: u32,
                        dst: &Arc<Texture>, dst_layer: u32) {
        assert_eq!(src.dim, dst.dim);
        let src_layout = vk::IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;
        let dst_layout = vk::IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
        self.set_image_layout(share, res, src, src_layer, src_layout);
        self.set_image_layout(share, res, dst, dst_layer, dst_layout);

        let regions = [vk::ImageCopy {
            srcSubresource: vk::ImageSubresourceLayers {
                aspectMask: vk::IMAGE_ASPECT_COLOR_BIT,
                mipLevel: 0,
                baseArrayLayer: src_layer,
                layerCount: 1,
            },
            srcOffset: vk::Offset3D {
                x: 0, y: 0, z: 0,
            },
            dstSubresource: vk::ImageSubresourceLayers {
                aspectMask: vk::IMAGE_ASPECT_COLOR_BIT,
                mipLevel: 0,
                baseArrayLayer: dst_layer,
                layerCount: 1,
            },
            dstOffset: vk::Offset3D {
                x: 0, y: 0, z: 0,
            },
            extent: vk::Extent3D {
                width: src.dim.w,
                height: src.dim.h,
                depth: src.dim.d,
            },
        }];
        unsafe {
            share.vk.CmdCopyImage(self.inner,
                                  src.inner, src_layout,
                                  dst.inner, dst_layout,
                                  regions.len() as u32,
                                  regions.as_ptr());
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Dimensions {
    pub w: u32,
    pub h: u32,
    pub d: u32,
}

impl From<vk::Extent3D> for Dimensions {
    fn from(ext: vk::Extent3D) -> Dimensions {
        Dimensions {
            w: ext.width,
            h: ext.height,
            d: ext.depth,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Texture {
    inner: vk::Image,
    memory: vk::DeviceMemory,
    default_layout: vk::ImageLayout,
    dim: Dimensions,
    usage: vk::ImageUsageFlagBits,
}

impl Texture {
    fn get_layer_size(&self) -> u32 {
        let bpp = 4; //TODO
        bpp * self.dim.w * self.dim.h * self.dim.d
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TargetView {
    inner: vk::ImageView,
    layer: u32,
    texture: Arc<Texture>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TargetSet {
    pub colors: Vec<TargetView>,
    pub depth: Option<TargetView>,
    pub stencil: Option<TargetView>,
}

pub struct SwapChain {
    gpu_texture: Arc<Texture>,
    cpu_texture: Arc<Texture>,
    cpu_layer_count: u32,
    cpu_current_layer: u32,
    views: Vec<TargetView>,
}

impl SwapChain {
    pub fn get_targets(&self) -> Vec<TargetView> {
        self.views.clone()
    }

    pub fn get_dimensions(&self) -> Dimensions {
        self.gpu_texture.dim.clone()
    }

    pub fn fetch_frame(&mut self, share: &Share, res: &mut ResourceState,
                       com: &CommandBuffer, frame_index: u32) {
        println!("fetching frame {}", frame_index); //TEMP
        self.cpu_current_layer += 1;
        if self.cpu_current_layer >= self.cpu_layer_count {
            self.cpu_current_layer = 0;
        }
        com.copy_texture(share, res, &self.gpu_texture, frame_index,
                         &self.cpu_texture, self.cpu_current_layer);
    }
}

pub struct Queue {
    inner: vk::Queue,
    family_index: u32,
    command_pool: vk::CommandPool,
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

impl Device {
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
        Queue {
            inner: queue,
            family_index: family_id,
            command_pool: com_pool,
        }
    }

    pub fn make_command_buffer(&self, queue: &Queue) -> CommandBuffer {
        let info = vk::CommandBufferAllocateInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
            pNext: ptr::null(),
            commandPool: queue.command_pool,
            level: vk::COMMAND_BUFFER_LEVEL_PRIMARY,
            commandBufferCount: 1,
        };
        let mut buf = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.share.vk.AllocateCommandBuffers(self.inner, &info, &mut buf)
        });
        CommandBuffer {
            inner: buf,
            family_index: queue.family_index,
        }
    }

    pub fn execute(&self, queue: &Queue, com: &CommandBuffer) {
        assert_eq!(queue.family_index, com.family_index);
        let vk = &self.share.vk;
        assert_eq!(vk::SUCCESS, unsafe {
            vk.EndCommandBuffer(com.inner)
        });
        let submit_info = vk::SubmitInfo {
            sType: vk::STRUCTURE_TYPE_SUBMIT_INFO,
            commandBufferCount: 1,
            pCommandBuffers: &com.inner,
            .. unsafe { mem::zeroed() }
        };
        assert_eq!(vk::SUCCESS, unsafe {
            vk.QueueSubmit(queue.inner, 1, &submit_info, 0)
        });
    }
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

    pub fn make_swap_chain(&self, width: u32, height: u32, count: u32) -> SwapChain {
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
                arrayLayers: count,
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
                arrayLayers: count,
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
            })
        };
        let views = (0 .. count).map(|i| {
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
            cpu_layer_count: count,
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
