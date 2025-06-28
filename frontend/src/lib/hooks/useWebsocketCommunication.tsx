import { useContext } from "preact/hooks"
import { WebsocketContext } from "../context/websocket.context"


export const useWebsocketCommunication = () => {

    const { send } = useContext(WebsocketContext);

    return send
}