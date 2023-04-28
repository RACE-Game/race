import React, { FC } from 'react';
import { FacadeWalletContext, DEFAULT_WALLET } from './facade-wallet-context';

const FacadeWalletWrapper: FC<{ children: React.ReactNode }> = ({ children }) =>
    <FacadeWalletContext.Provider value={DEFAULT_WALLET}>
        {children}
    </FacadeWalletContext.Provider>

export default FacadeWalletWrapper;
