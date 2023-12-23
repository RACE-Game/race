import { PlayerProfile } from './accounts';
import { ITransport } from './transport';

export class ProfileCache {
  transport: ITransport;
  caches: Map<string, PlayerProfile>;

  constructor(transport: ITransport) {
    this.transport = transport;
    this.caches = new Map();
  }

  async getProfile(playerAddr: string): Promise<PlayerProfile | undefined> {
    let exist = this.caches.get(playerAddr);
    if (exist !== undefined) {
      return exist;
    } else {
      const p = await this.transport.getPlayerProfile(playerAddr);
      if (p === undefined) {
        console.warn('Failed to fetch the profile of %s', playerAddr);
        return undefined;
      } else {
        this.caches.set(playerAddr, p);
        return p;
      }
    }
  }
}
