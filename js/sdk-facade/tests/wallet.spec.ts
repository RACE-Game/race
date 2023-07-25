import { assert } from 'chai';
import { FacadeWallet } from '../src/facade-wallet';

describe('Test FacadeWallet', () => {
  it('Test creation', () => {
    const w = new FacadeWallet();
    assert.equal(w.isConnected, true);
    assert.equal(typeof w.walletAddr, 'string');
  });
});
