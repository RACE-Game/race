import React, { useState } from 'react'
import Sidemenu from './Sidemenu';
import Profile from './Profile';
import Container from './Container';
import init, { AppClient, AppHelper } from 'race-sdk';
import './App.css'
import ProfileContext from './profile-context';
import HelperContext from './helper-context';
import GameContext, { GameContextData } from './game-context';
import Header from './Header';
import Logs from './Logs';

class App extends React.Component {

  state: {
    helper: AppHelper | undefined,
    profile: any | undefined,
    gameAddr: string | undefined,
    context: any | undefined,
    client: AppClient | undefined,
  }

  constructor(props: any) {
    super(props);

    this.setProfile = this.setProfile.bind(this);
    this.setContext = this.setContext.bind(this);
    this.loadExample = this.loadExample.bind(this);
    this.setClient = this.setClient.bind(this);

    this.state = {
      helper: undefined,
      profile: undefined,
      gameAddr: undefined,
      context: undefined,
      client: undefined,
    }
  }

  componentDidMount() {
    this.initHelper();
  }

  setProfile(profile: any) {
    this.setState({ profile })
  }

  setContext(context: any) {
    this.setState({ context })
  }

  setClient(client: AppClient) {
    this.setState({ client })
  }

  loadExample(addr: string) {
    this.setState({ gameAddr: addr })
  }

  initHelper() {
    init().then(instance => {
      // console.log("SDK loaded, ", instance);
      AppHelper.try_init('facade', 'ws://localhost:12002').then(helper => {
        console.log("Helper created, ", helper);
        this.setState({ helper: helper })
      })
    })
  }

  render() {
    return (
      <HelperContext.Provider value={this.state.helper}>
        <ProfileContext.Provider value={{ profile: this.state.profile, setProfile: this.setProfile }}>
          <GameContext.Provider value={{
            context: this.state.context,
            setContext: this.setContext,
            client: this.state.client,
            setClient: this.setClient
          }}>
            <div className="w-screen max-w-7xl min-h-screen grid grid-cols-4 grid-rows-6 p-4 gap-2">
              <div className="row-span-6">
                <Sidemenu onSelect={this.loadExample} />
              </div>
              <div className="col-span-2">
                <Header />
              </div>
              <Profile />

              <div className="row-span-6 col-span-2">
                <Container gameAddr={this.state.gameAddr} />
              </div>
              <div className="row-span-5">
                <Logs />
              </div>
            </div>
          </GameContext.Provider>
        </ProfileContext.Provider>
      </HelperContext.Provider >
    )
  }
}

export default App
