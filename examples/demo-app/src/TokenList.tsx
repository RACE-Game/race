import { useContext, useState, useEffect, FC } from 'react';
import { HelperContext } from './helper-context';
import { AppHelper, INft, IToken, PlayerProfile, TokenWithBalance } from '@race-foundation/sdk-core';
import { ProfileContext } from './profile-context';

function formatBalance(token: IToken | TokenWithBalance): string {
    if ('amount' in token) {
        const amt = Number(token.amount) / Math.pow(10, token.decimals);
        return '' + amt;
    } else {
        return '--';
    }
}

const TokenList: FC = () => {
    const helper = useContext<AppHelper | undefined>(HelperContext);
    const profile = useContext<PlayerProfile | undefined>(ProfileContext);
    const [tokens, setTokens] = useState<(IToken | TokenWithBalance)[]>([]);
    const [nfts, setNfts] = useState<INft[]>([]);

    useEffect(() => {
        if (helper !== undefined && profile !== undefined) {
            // Fetch tokens
            (async () => {
                const tokens = await helper.listTokensWithBalance(profile.addr);
                setTokens(tokens);
                const nfts = await helper.listNfts(profile.addr);
                console.log(nfts);
                setNfts(nfts);
            })();
        } else if (helper !== undefined) {
            // Fetch tokens with balances
            (async () => {
                const tokens = await helper.listTokens();
                setTokens(tokens);
            })();
        }
    }, [helper, profile]);

    return (
        <div className="flex flex-col h-full w-full border border-gray-500 overflow-y-scroll p-4">
            <div> NFT: {nfts.length}</div>
            <div className="uppercase font-bold">Tokens</div>
            {
                tokens.map(token => (
                    <div key={token.addr} className="flex items-center py-1 px-1">
                        <img className="w-5 h-5 mr-2" src={token.icon} />
                        <div>
                            <div>{token.symbol}</div>
                            <div className="text-gray-400 text-xs">{token.name}</div>
                        </div>
                        <div className="flex-1 flex justify-end items-center">{formatBalance(token)}</div>
                    </div>
                ))
            }
        </div>
    )
}

export default TokenList;
