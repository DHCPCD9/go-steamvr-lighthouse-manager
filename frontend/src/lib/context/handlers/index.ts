import type { EventHandlerMap } from './types';
import { lighthouseFoundHandler, lighthouseUpdateHandler } from './lighthouse';
import { 
    groupCreateHandler, 
    groupRenameHandler, 
    groupDeleteHandler, 
    groupLighthousesAddedHandler, 
    groupUpdateFlagsHandler 
} from './group';
import { clientConfigureHandler, clientPlatformHandler, clientScanHandler } from './client';
import { steamvrStatusHandler } from './steamvr';

export const eventHandlers: EventHandlerMap = {
    'lighthouse.found': lighthouseFoundHandler,
    'lighthouse.update': lighthouseUpdateHandler,
    'group.create': groupCreateHandler,
    'group.rename': groupRenameHandler,
    'group.delete': groupDeleteHandler,
    'groups.lighthouses.added': groupLighthousesAddedHandler,
    'group.update.flags': groupUpdateFlagsHandler,
    'client.configure': clientConfigureHandler,
    'client.platform': clientPlatformHandler,
    'client.scan': clientScanHandler,
    'steamvr.status': steamvrStatusHandler,
};
