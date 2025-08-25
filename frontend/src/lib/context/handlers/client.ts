import type { EventHandler } from './types';

export const clientConfigureHandler: EventHandler = (data) => {
    if (!data?.data) return;
    
    return (prevState) => ({
        ...prevState,
        configuration: data.data
    });
};

export const clientPlatformHandler: EventHandler = (data) => {
    if (!data?.data) return;
    
    return (prevState) => ({
        ...prevState,
        platform: data.data
    });
};

export const clientScanHandler: EventHandler = (data) => {
    if (!data?.data) return;
    
    return (prevState) => ({
        ...prevState,
        scanning: data.data.status
    });
};

export const clientUpdateFoundHandler: EventHandler = (data) => {
    if (!data?.data) return;
    
    return (prevState) => ({
        ...prevState,
        update: data.data
    });
}