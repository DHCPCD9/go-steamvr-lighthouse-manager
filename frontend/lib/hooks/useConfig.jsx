import { useContext, useEffect, useState } from "preact/hooks"
import { WebsocketContext } from "../context/websocket.context"


export const useConfig = () => {

    const { configuration } = useContext(WebsocketContext);

    return configuration
}