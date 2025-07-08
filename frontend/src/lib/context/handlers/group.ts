import type { EventHandler } from './types';

export const groupCreateHandler: EventHandler = (data) => {
    if (!data?.data) return;
    
    return (prevState) => ({
        ...prevState,
        groups: {
            ...prevState.groups,
            [data.id]: data.data
        }
    });
};

export const groupRenameHandler: EventHandler = (data) => {
    if (!data?.data) return;
    
    return (prevState) => ({
        ...prevState,
        groups: {
            ...prevState.groups,
            [data.data.id]: {
                ...prevState.groups[data.data.id],
                name: data.data.name
            }
        }
    });
};

export const groupDeleteHandler: EventHandler = (data) => {
    if (!data?.data) return;
    
    return (prevState) => {
        const newGroups = { ...prevState.groups };
        delete newGroups[data.data.id];
        return {
            ...prevState,
            groups: newGroups
        };
    };
};

export const groupLighthousesAddedHandler: EventHandler = (data) => {
    if (!data?.data) return;
    
    return (prevState) => {
        const group = prevState.groups[data.data.group];
        if (!group) return prevState;
        
        return {
            ...prevState,
            groups: {
                ...prevState.groups,
                [data.data.group]: {
                    ...group,
                    base_stations: [...group.base_stations, data.data.id]
                }
            }
        };
    };
};

export const groupUpdateFlagsHandler: EventHandler = (data) => {
    if (!data?.data) return;
    
    return (prevState) => ({
        ...prevState,
        groups: {
            ...prevState.groups,
            [data.data.id]: {
                ...prevState.groups[data.data.id],
                managed_flags: data.data.flags
            }
        }
    });
};