import { create } from 'zustand'

export const useContainerTitlebar = create((set) => ({
    items: [],
    callbackOnClick: undefined,
    callbackOnLastClick: undefined,
    setItems: (newItems) => set({ items: newItems }),
    setCallback: (callback) => set({ callbackOnClick: callback }),
    setCallbackOnLast: (callback) => set({ callbackOnLastClick: callback })
}));