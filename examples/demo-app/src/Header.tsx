import { useState, useEffect, useContext } from "react";
import { HelperContext } from "./helper-context";
import { GameAccount } from "@race-foundation/sdk-core";

export type JoinHandler = (amount: bigint) => void;
export type ExitHandler = () => void;

type FormData = {
    amount: bigint;
};

function Header(props: { gameAddr: string, onJoin?: JoinHandler, onExit?: ExitHandler }) {
    const { gameAddr, onJoin, onExit } = props;
    const [form, setForm] = useState<FormData>({ amount: 100n });
    let [account, setAccount] = useState<GameAccount | undefined>(undefined);
    let helper = useContext(HelperContext);

    // timer
    useEffect(() => {
        let t = setInterval(async () => {
            if (helper !== undefined) {
                let account = await helper.getGame(gameAddr);
                setAccount(account);
            }
        }, 1000);
        return () => clearInterval(t);
    });

    if (account === null) {
        return (
            <div> Not connected! </div>
        );
    } else if (account === undefined) {
        return (
            <div className="w-full h-32 p-2 flex flex-wrap"> Loading </div>
        );
    } else {

        let joinButton = null;
        if (onJoin !== undefined) {
            const f = () => {
                onJoin(form.amount);
            };
            const c = (e: React.ChangeEvent<HTMLInputElement>) => {
                setForm({ amount: BigInt(e.target.value) });
            }
            joinButton = (
                <div className="m-2">
                    <input className="border border-black" value={'' + form.amount} onChange={c} />
                    <button className="border bg-black text-white px-2 ml-4" onClick={f}>Join</button>
                </div>
            )
        }

        let exitButton = onExit && (
            <div className="m-2">
                <button className="border bg-black text-white px-2 ml-4" onClick={onExit}>Join</button>
            </div>
        );

        return (
            <div className="w-full h-32 p-2 flex flex-wrap">
                <div className="m-2"> <span className="font-bold">Address:</span> {account.addr}</div>
                <div className="m-2"> <span className="font-bold">Status:</span> {account.players.length}</div>
                <div className="m-2"> <span className="font-bold">Servers:</span> {account.servers.length}</div>
                <div className="m-2"> <span className="font-bold">Settles:</span> {'' + account.settleVersion}</div>
                <div className="m-2"> <span className="font-bold">Accesses:</span> {'' + account.accessVersion}</div>
                {joinButton}
                {exitButton}
            </div>
        );
    }
}

export default Header;
