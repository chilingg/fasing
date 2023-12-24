
export function getDirection(p1, p2) {
    if (p2[0] < p1[0] && p2[1] > p1[1]) {
        return '1';
    } else if (p2[0] < p1[0] && p2[1] === p1[1]) {
        return '4';
    } else if (p2[0] < p1[0] && p2[1] < p1[1]) {
        return '7';
    } else if (p2[0] === p1[0] && p2[1] > p1[1]) {
        return '2';
    } else if (p2[0] === p1[0] && p2[1] === p1[1]) {
        return '5';
    } else if (p2[0] === p1[0] && p2[1] < p1[1]) {
        return '8';
    } else if (p2[0] > p1[0] && p2[1] > p1[1]) {
        return '3';
    } else if (p2[0] > p1[0] && p2[1] === p1[1]) {
        return '6';
    } else if (p2[0] > p1[0] && p2[1] < p1[1]) {
        return '9';
    } else {
        throw new Error(`Undefine direction in ${p1} → ${p2}`);
    }
}

export function is_diagonal(dir) {
    switch (dir) {
        case '1':
        case '3':
        case '7':
        case '9':
            return true;
        default:
            return false;
    }
}

export function getPathInfo(path) {
    return path.map((p, i) => {
        let info = { pos: [...p] };
        if (i != 0) {
            let prePos = [...path[i - 1]];
            info.pre = { pos: prePos, dir: getDirection(prePos, p) };
        }

        let next = path[i + 1];
        if (next) {
            info.next = { pos: [...next] }
            info.next.dir = getDirection(p, next);
        }

        return info;
    })
}
