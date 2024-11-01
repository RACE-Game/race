import { useContext, useState, useEffect, FC } from 'react';
import { HelperContext } from './helper-context';
import { AppHelper, INft, IToken, ITokenWithBalance, PlayerProfile, TokenWithBalance } from '@race-foundation/sdk-core';
import { ProfileContext } from './profile-context';
import { Chain } from './types';
import { FAVORED_TOKEN_LIST } from './constants';

function formatBalance(token: IToken | TokenWithBalance): string {
  if ('amount' in token) {
    const amt = Number(token.amount) / Math.pow(10, token.decimals);
    return '' + amt;
  } else {
    return '--';
  }
}

type TokenListProps = {chain: Chain}

const TokenList: FC<TokenListProps> = (props: TokenListProps) => {
  const helper = useContext<AppHelper | undefined>(HelperContext);
  const profile = useContext<PlayerProfile | undefined>(ProfileContext);
  const [tokens, setTokens] = useState<(IToken | ITokenWithBalance)[]>([]);
  const [nfts, setNfts] = useState<INft[]>([]);
  const favoredTokenList = FAVORED_TOKEN_LIST[props.chain]

  useEffect(() => {
    if (helper !== undefined && profile !== undefined) {
      // Fetch tokens
      (async () => {
        const tokens = await helper.listTokensWithBalance(profile.addr, favoredTokenList);
        setTokens(tokens);
        const nfts = await helper.listNfts(profile.addr);
        console.log(nfts);
        setNfts(nfts);
      })();
    } else if (helper !== undefined) {
      // Fetch tokens with balances
      (async () => {
        const tokens = await helper.listTokens(favoredTokenList);
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
