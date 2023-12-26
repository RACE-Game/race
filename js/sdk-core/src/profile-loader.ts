import { PlayerProfile } from './accounts';
import { PlayerProfileWithPfp, ProfileCallbackFunction } from './app-client';
import { IStorage } from './storage';
import { ITransport } from './transport';

type LoadingStatus = 'loading' | 'loaded' | 'failed';

export class ProfileLoader {
  transport: ITransport;
  caches: Map<string, PlayerProfileWithPfp>;
  loadingStatus: Map<string, LoadingStatus>;
  storage?: IStorage;
  shutdown: boolean;
  onProfile?: ProfileCallbackFunction;

  constructor(transport: ITransport, storage: IStorage | undefined, onProfile: ProfileCallbackFunction | undefined) {
    this.transport = transport;
    this.storage = storage;
    this.caches = new Map();
    this.loadingStatus = new Map();
    this.shutdown = false;
    this.onProfile = onProfile;
  }

  async __loadProfile(playerAddr: string): Promise<PlayerProfileWithPfp | undefined> {
    const profile = await this.transport.getPlayerProfile(playerAddr);
    if (profile === undefined) {
      return undefined;
    } else {
      let p;
      if (profile.pfp !== undefined) {
        let pfp = await this.transport.getNft(profile.addr, this.storage);
        p = { pfp, addr: profile.addr, nick: profile.nick };
      } else {
        p = { pfp: undefined, addr: profile.addr, nick: profile.nick };
      }
      return p;
    }
  }

  getProfile(playerAddr: string): PlayerProfileWithPfp | undefined {
    return this.caches.get(playerAddr);
  }

  async start() {
    while (true) {
      if (this.shutdown) {
        break;
      }
      for (const [addr, s] of this.loadingStatus) {
        if (s === 'loading') {
          const p = await this.__loadProfile(addr);
          if (p === undefined) {
            this.loadingStatus.set(addr, 'failed');
          } else {
            if (this.onProfile !== undefined) {
              this.onProfile(p);
            }
            this.caches.set(addr, p);
            this.loadingStatus.set(addr, 'loaded');
          }
        }
      }
      await new Promise(r => setTimeout(r, 1000));
    }
  }

  load(playerAddr: string) {
    if (!this.loadingStatus.has(playerAddr)) {
      this.loadingStatus.set(playerAddr, 'loading');
    }
  }

  stop() {
    this.shutdown = true;
  }
}
