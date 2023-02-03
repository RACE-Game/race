import React, { useState } from 'react'
import Sidemenu from './Sidemenu';
import Profile from './Profile';
import Container from './Container';
import init, { AppHelper } from 'race-sdk';
import './App.css'
import ProfileContext from './profile-context';
import HelperContext from './helper-context';

class App extends React.Component {

  state: {
    helper: AppHelper | undefined,
    profile: any | undefined,
    gameAddr: string | undefined,
  }

  constructor(props: any) {
    super(props);

    this.setProfile = this.setProfile.bind(this);
    this.loadExample = this.loadExample.bind(this);

    this.state = {
      helper: undefined,
      profile: undefined,
      gameAddr: undefined,
    }
  }

  componentDidMount() {
    this.initHelper();
  }

  setProfile(profile: any) {
    this.setState({ profile })
  }

  loadExample(addr: string) {
    this.setState({ gameAddr: addr })
  }

  initHelper() {
    init().then(instance => {
      console.log("SDK loaded, ", instance);
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
          <div className="w-screen max-w-7xl min-h-screen grid grid-cols-4 grid-rows-6 p-4">
            <div className="row-span-6">
              <Sidemenu onSelect={this.loadExample} />
            </div>
            <div className="row-span-6 col-span-2">
              <Container gameAddr={this.state.gameAddr} />
            </div>
            <Profile />
          </div>
        </ProfileContext.Provider>
      </HelperContext.Provider >
    )
  }
}

export default App
