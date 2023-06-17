import { useContext, useState, useEffect, FC } from 'react';
import { HelperContext } from './helper-context';
import { AppHelper, IToken, PlayerProfile } from '@race-foundation/sdk-core';
import { ProfileContext } from './profile-context';

function formatBalance(token: IToken, balance: bigint | undefined): string {
    if (balance === undefined) return '--';
    const amt = Number(balance) / Math.pow(10, token.decimals);
    return '' + amt;
}

const TokenList: FC = () => {
    const helper = useContext<AppHelper | undefined>(HelperContext);
    const profile = useContext<PlayerProfile | undefined>(ProfileContext);
    const [tokens, setTokens] = useState<IToken[]>([]);
    const [balances, setBalances] = useState<Map<string, bigint>>(new Map());

    useEffect(() => {
        if (helper !== undefined && profile !== undefined) {
            const fetchTokens = async () => {
                console.log("fetch tokens")
                const tokens = await helper.listTokens();
                setTokens(tokens);
                const tokenAddrs = tokens.map(t => t.addr);
                const walletAddr = profile.addr;
                const balances = await helper.fetchBalances(walletAddr, tokenAddrs);
                setBalances(balances);
            };
            fetchTokens();
        }
    }, [helper, profile]);

    return (
        <div className="flex flex-col h-full w-full border border-gray-500 overflow-y-scroll p-4">
            <div className="uppercase font-bold">Tokens</div>
            {
                tokens.map(token => (
                    <div key={token.addr} className="flex items-center py-4 px-1">
                        <img className="w-5 h-5 mr-2" src={token.icon} />
                        <div>
                            <div>{token.symbol}</div>
                            <div className="text-gray-400">{token.name}</div>
                        </div>
                        <div className="flex-1 flex justify-end items-center">{formatBalance(token, balances.get(token.addr))}</div>
                    </div>
                ))
            }
        </div>
    )
}

export default TokenList;
