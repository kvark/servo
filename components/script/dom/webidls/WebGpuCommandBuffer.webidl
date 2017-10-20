/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

typedef unsigned long WebGpuVertexCount;
typedef unsigned long WebGpuIndexCount;
typedef unsigned long WebGpuInstanceCount;

enum WebGpuClearValueKind {
	"ColorUint",
	"ColorInt",
	"ColorFloat",
	"DepthStencil",
};

dictionary WebGpuBufferState {
	required WebGpuBufferAccess access;
};

dictionary WebGpuImageState {
	required WebGpuImageAccess access;
	required WebGpuImageLayout layout;
};

dictionary WebGpuBufferBarrier {
	required WebGpuBufferState stateSrc;
	required WebGpuBufferState stateDst;
	required WebGpuBuffer target;
	//TODO: offset/size
};

dictionary WebGpuImageBarrier {
	required WebGpuImageState stateSrc;
	required WebGpuImageState stateDst;
	required WebGpuImage target;
	//TODO: subresource range
};

dictionary WebGpuRectangle {
	required unsigned short x;
	required unsigned short y;
	required unsigned short width;
	required unsigned short height;
};

dictionary WebGpuViewport {
	required WebGpuRectangle rect;
	required float near;
	required float far;
};

dictionary WebGpuClearValue {
	required WebGpuClearValueKind kind;
	required sequence<float> data;
};

dictionary WebGpuImageOffset {
	unsigned long x = 0;
	unsigned long y = 0;
	unsigned long z = 0;
};

dictionary WebGpuImageExtent {
	required unsigned long width;
	required unsigned long height;
	unsigned long depth = 1;
};

dictionary WebGpuBufferImageCopy {
	required unsigned long bufferOffset;
	required unsigned long bufferRowPitch;
	required unsigned long bufferSlicePitch;
	//required WebGpuImageAspect imageAspect,
	//TODO: image::SubresourceLayers,
	required WebGpuImageOffset imageOffset;
	required WebGpuImageExtent imageExtent;
};


interface WebGpuCommandBuffer {
	void begin();
	void finish();

	void pipelineBarrier(
		WebGpuPipelineStage srcStages,
		WebGpuPipelineStage dstStages,
		sequence<WebGpuBufferBarrier> buffers,
		sequence<WebGpuImageBarrier> images
	);

	void copyBufferToImage(
		WebGpuBuffer src,
		WebGpuImage dst,
		WebGpuImageLayout dstLayout,
		sequence<WebGpuBufferImageCopy> regions
	);

	void copyImageToBuffer(
		WebGpuImage src,
		WebGpuImageLayout srcLayout,
		WebGpuBuffer dst,
		sequence<WebGpuBufferImageCopy> regions
	);

	void beginRenderPass(
		WebGpuRenderPass renderPass,
		WebGpuFramebuffer framebuffer,
		WebGpuRectangle rect,
		sequence<WebGpuClearValue> clearValues
	);

	void endRenderPass();

	void bindGraphicsPipeline(WebGpuGraphicsPipeline pipeline);

	void bindGraphicsDescriptorSets(
		WebGpuPipelineLayout layout,
		unsigned long descOffset,
		sequence<WebGpuDescriptorSet> descSets
	);

	void setScissors(sequence<WebGpuRectangle> rectangles);

	void setViewports(sequence<WebGpuViewport> viewports);

	void draw(
		WebGpuVertexCount start_vertex,
		WebGpuVertexCount vertex_count,
		WebGpuInstanceCount start_instance,
		WebGpuInstanceCount instance_count
	);
};
