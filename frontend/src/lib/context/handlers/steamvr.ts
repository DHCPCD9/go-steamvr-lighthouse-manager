import type { EventHandler } from './types';

export const steamvrStatusHandler: EventHandler = (data) => {
    if (!data?.data) return;
    
    return (prevState) => ({
        ...prevState,
        steamvr: { active: data.data.status }
    });
};