export async function smoothResize(targetWidth, targetHeight) {
    await window.runtime.WindowSetSize(targetWidth, targetHeight);
}