export async function smoothResize(targetWidth, targetHeight, duration = 500) {
    // Get initial window size
    // const { w, h } = await window.runtime.WindowGetSize();
    
    // // Calculate size differences
    // const deltaW = targetWidth - w;
    // const deltaH = targetHeight - h;
    
    // // Animation timing
    // const startTime = Date.now();
    // const endTime = startTime + duration;

    // // Animation loop
    // return new Promise(async (resolve) => {
    //     while (true) {
    //         const now = Date.now();
    //         const progress = Math.min((now - startTime) / duration, 1);
            
    //         // Calculate intermediate size
    //         const currentWidth = Math.round(w + deltaW * progress);
    //         const currentHeight = Math.round(h + deltaH * progress);
            
    //         // Update window size
    //         await window.runtime.WindowSetSize(currentWidth, currentHeight);
            
    //         // Check if animation should continue
    //         if (now >= endTime) {
    //             // Final resize to ensure exact target size
    //             await window.runtime.WindowSetSize(targetWidth, targetHeight);
    //             resolve();
    //             break;
    //         }
            
    //         // Add slight delay to prevent blocking
    //         await new Promise(r => setTimeout(r, 16));
    //     }
    // });

    await window.runtime.WindowSetSize(targetWidth, targetHeight);
}