import { create } from 'zustand'

interface ContainerTitleBar {
    items: Array<{ text: string, link: string }>,
    callbackOnClick: () => void | undefined,
    callbackOnLastClick: () => void | undefined,
    setItems: (newItems: Array<{ text: string, link: string }>) => void
    setCallback: (callback: () => void) => void
    setCallbackOnLast: (callback: () => void) => void
}

export const useContainerTitlebar = create<ContainerTitleBar>()((set) => ({
    items: [],
    callbackOnClick: undefined,
    callbackOnLastClick: undefined,
    setItems: (newItems: Array<{ text: string, link: string }>) => set({ items: newItems }),
    setCallback: (callback: () => void) => set({ callbackOnClick: callback }),
    setCallbackOnLast: (callback: () => void) => set({ callbackOnLastClick: callback })
}));