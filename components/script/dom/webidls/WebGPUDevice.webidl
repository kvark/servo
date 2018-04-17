/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
//
// WebGPU IDL definitions scraped from the WebGPU sketch:
//

dictionary WebGPUExtensions {
    required wg_bool anisotropicFiltering;
};

dictionary WebGPUFeatures {
    required wg_bool logicOp;
};

dictionary WebGPULimits {
    required wg_u32 maxBindGroups;
};

interface WebGPUDevice {
    WebGPUExtensions getExtensions();
    WebGPUFeatures getFeatures();
    WebGPULimits getLimits();

    //WebGPUBuffer createBuffer(WebGPUBufferDescriptor descriptor);
    //WebGPUTexture createTexture(WebGPUTextureDescriptor descriptor);
    //WebGPUSampler createSampler(WebGPUSamplerDescriptor descriptor);

    //WebGPUBindGroupLayout createBindGroupLayout(WebGPUBindGroupLayoutDescriptor descriptor);
    //WebGPUPipelineLayout createPipelineLayout(WebGPUPipelineLayoutDescriptor descriptor);
    //WebGPUBindGroup createBindGroup(WebGPUBindGroupDescriptor descriptor);

    //WebGPUBlendState createBlendState(WebGPUBlendStateDescriptor descriptor);
    //WebGPUDepthStencilState createDepthStencilState(WebGPUDepthStencilStateDescriptor descriptor);
    //WebGPUInputState createInputState(WebGPUInputStateDescriptor descriptor);
    //WebGPUShaderModule createShaderModule(WebGPUShaderModuleDescriptor descriptor);
    //WebGPUAttachmentState createAttachmentState(WebGPUAttachmentStateDescriptor descriptor);
    //WebGPUComputePipeline createComputePipeline(WebGPUComputePipelineDescriptor descriptor);
    //WebGPURenderPipeline createRenderPipeline(WebGPURenderPipelineDescriptor descriptor);

    //WebGPUCommandEncoder createCommandEncoder(WebGPUCommandEncoderDescriptor descriptor);
    //WebGPUFence createFence(WebGPUFenceDescriptor descriptor);

    //WebGPUQueue getQueue();
};
