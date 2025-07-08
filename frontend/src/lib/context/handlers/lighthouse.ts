import type { EventHandler } from './types';

export const lighthouseFoundHandler: EventHandler = (data) => {
    if (!data?.data) return;
    
    return (prevState) => ({
        ...prevState,
        lighthouses: {
            ...prevState.lighthouses,
            [data.data.id]: data.data
        }
    });
};

export const lighthouseUpdateHandler: EventHandler = (data) => {
    if (!data?.data) return;
    
    return (prevState) => ({
        ...prevState,
        lighthouses: {
            ...prevState.lighthouses,
            [data.data.id]: {
                ...prevState.lighthouses[data.data.id],
                [data.data.field_name]: data.data.value
            }
        }
    });
};
