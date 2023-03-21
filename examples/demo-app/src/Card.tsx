// A common poker card component

interface CardProps {
    value: string | null
}

function parseSuit(value: string) {
    let suitStr = value[0];
    switch (suitStr) {
        case 'h':
            return '♥';
        case 'c':
            return '♣';
        case 'd':
            return '♦';
        case 's':
            return '♠';
    }
}
function parseKind(value: string) {
    let kindStr = value[1];
    switch (kindStr) {
        case 'a':
            return 'A';
        case '2':
            return '2';
        case '3':
            return '3';
        case '4':
            return '4';
        case '5':
            return '5';
        case '6':
            return '6';
        case '7':
            return '7';
        case '8':
            return '8';
        case '9':
            return '9';
        case 't':
            return '10';
        case 'j':
            return 'J';
        case 'q':
            return 'Q';
        case 'k':
            return 'K';
    }
}

function Card(props: CardProps) {
    if (typeof props.value === 'string') {
        return <div className="w-20 h-32 border border-black rounded-sm flex flex-col justify-between p-4">
            <div className="self-start text-[2rem]">{parseKind(props.value)}</div>
            <div className="self-end text-[3rem]">{parseSuit(props.value)}</div>
        </div>
    } else {
        // render a card back
        return <div className="w-20 h-32 border border-black rounded-sm flex flex-col justify-between p-4 bg-gray-300">
        </div>
    }
}

export default Card;
