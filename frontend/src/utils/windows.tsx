export async function smoothResize(targetWidth, targetHeight) {
    //@ts-ignore
    await window.runtime.WindowSetSize(targetWidth, targetHeight);
}