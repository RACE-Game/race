import { useState, useEffect, useContext } from "react";
import HelperContext from "./helper-context";

function Header() {
  let [account, setAccount] = useState<any | undefined>(undefined);
  let helper = useContext(HelperContext);

  // timer
  useEffect(() => {
    let t = setInterval(async () => {
      if (helper !== undefined) {
        // let account = await helper.get_game_account("EXAMPLE_RAFFLE_ADDRESS");
        // setAccount(account);
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
      <div> Loading </div>
    );
  }else {
    return (
      <div className="w-full h-full p-2 flex flex-wrap">
        <div className="m-2"> <span className="font-bold">Address:</span> {account.game_addr}</div>
        <div className="m-2"> <span className="font-bold">Status:</span> {account.players.length}</div>
        <div className="m-2"> <span className="font-bold">Servers:</span> {account.servers.length}</div>
        <div className="m-2"> <span className="font-bold">Settles:</span> {account.settle_version}</div>
        <div className="m-2"> <span className="font-bold">Accesses:</span> {account.access_version}</div>
      </div>
    );
  }
}

export default Header;
