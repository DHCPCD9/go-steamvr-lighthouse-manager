import { useContext, useEffect, useState } from "preact/hooks"
import { WebsocketContext } from "../context/websocket.context"


export const useSteamVRStatus = () => {

    const { steamvr } = useContext(WebsocketContext);

    return steamvr.active
}