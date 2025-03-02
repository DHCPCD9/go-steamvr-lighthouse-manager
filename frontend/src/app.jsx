import './app.css';
import { ChangeBaseStationChannel, GetFoundBaseStations, InitBluetooth } from "../wailsjs/go/main/App";
import { useEffect, useState } from "preact/hooks";
import { LoaderIcon } from './components/Loader';
import { BaseStation } from './components/BaseStation';
import { ToastContainer } from 'react-toastify'
export function App(props) {


    const [warning, setWarning] = useState("");
    const [status, setStatus] = useState("");
    const [baseStations, setBaseStations] = useState([]);

    const reScan = () => {
        setStatus("scan");
        setTimeout(() => {
            setStatus("idle");
        }, 3000);
    }

    useEffect(() => {
        (async () => {
            let bluetoothStatus = await InitBluetooth();

            if (!bluetoothStatus) {
                setWarning("Unable to enable bluetooth.")
                setStatus("error");
                return;
            }

            setStatus("scan");
            setTimeout(() => {
                setStatus("idle");
            }, 3000);
        })()
    }, []);

    useEffect(() => {
        let interval;
        if (status == "scan") {
            //Requesting every couple of seconds base stations that it found.
            interval = setInterval(async () => {
                let foundBaseStations = await GetFoundBaseStations();
                setBaseStations(foundBaseStations);
            }, 300);
        }



        return () => clearInterval(interval);
    }, [status]);

    const setChannel = async (bs, channel) => {
        if (channel < 1 || channel > 16) {
            return {
                ok: false,
                message: "Channel range exceeded"
            }
        }

        console.log(baseStations)
        if (baseStations.find(c => c.Channel == channel)) {
            return {
                ok: false,
                message: "This channel conflicts with another base station"
            }
        }

        let c = await ChangeBaseStationChannel(bs, channel);

        if (c.startsWith("error")) return alert(c);
        
        return {
            ok: true,
            message: c
        };
    }

    const updateBaseStation = async (bs, update) => {
        let i = baseStations.findIndex(c => c.Name == bs);
        let bStations = baseStations;
        bStations[i] = {...baseStations[i], ...update};

        console.log(bStations)
        setBaseStations(bStations);
    }



    return (
        <div className='px-2 text-white py-4 flex flex-col gap-2 --wails-draggable:drag'>
            <ToastContainer
            position="bottom-center"
            autoClose={5000}
            hideProgressBar={false}
            newestOnTop={false}
            closeOnClick={false}
            rtl={false}
            pauseOnFocusLoss
            draggable
            pauseOnHover
            limit={2}
            theme="dark"
            />
            <div className='flex flex-col gap-2'>
                {baseStations.map((bs, i) =>
                    <BaseStation key={i} station={bs} rescan={reScan} setChannel={setChannel} updateBaseStation={updateBaseStation}/>
                )}
            </div>

            {status != "scan" && <button onClick={reScan} className='text-[#5835F3] h-[48px] w-full bg-[#5835F3]/10 border-[2px]  border-[#5835F3] rounded-[6px]'>Refresh</button> }

            {status == "scan" && <div className='flex flex-col items-center text-[16px]'>
                    <LoaderIcon /> Scanning base stations, please wait
            </div>}

        </div>
    )
}
