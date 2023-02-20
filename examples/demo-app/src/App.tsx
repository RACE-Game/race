import React, { useState, useEffect } from 'react'
import { Outlet } from 'react-router-dom';
import Sidemenu from './Sidemenu';
import Profile from './Profile';
import init, { AppHelper, Event } from 'race-sdk';
import './App.css'
import ProfileContext, { ProfileData } from './profile-context';
import LogsContext from './logs-context';
import HelperContext from './helper-context';
import Header from './Header';
import Logs from './Logs';

function App() {

  const [helper, setHelper] = useState<AppHelper | undefined>(undefined);
  const [profile, setProfile] = useState<ProfileData | undefined>(undefined);
  let [logs, setLogs] = useState<Array<Event>>([]);

  const addLog = (event: Event) => {
    setLogs(logs => {
      let newLogs = [...logs, event];
      if (newLogs.length > 30) {
        newLogs.shift();
      }
      return newLogs;
    });
  }

  useEffect(() => {
    const initHelper = async () => {
      await init();
      let client = await AppHelper.try_init('facade', 'ws://localhost:12002');
      console.log("AppHelper initialized");
      setHelper(client);
    }
    initHelper();
  }, []);

  return (
    <HelperContext.Provider value={helper}>
      <ProfileContext.Provider value={profile}>
        <LogsContext.Provider value={{ addLog }}>
          <div className="w-screen max-w-7xl min-h-screen grid grid-cols-4 grid-rows-6 p-4 gap-2">
            <div className="row-span-6">
              <Sidemenu />
            </div>
            <div className="col-span-2">
              <Header />
            </div>
            <Profile updateProfile={setProfile} />
            <div className="row-span-6 col-span-2">
              <Outlet />
            </div>
            <div className="row-span-5">
              <Logs logs={logs} />
            </div>
          </div>
        </LogsContext.Provider>
      </ProfileContext.Provider>
    </HelperContext.Provider>
  );
}

export default App;
