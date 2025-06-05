import ky from "ky";
import asyncTimeout from "./asyncTimeout";

let error_limit = 100;
let error_reset = 60;

const esi = ky.create({
    prefixUrl: "https://esi.evetech.net/latest/",
    retry: 0,
    hooks: {
        beforeRequest: [
            async () => {
                if (error_limit < 10) {
                    await asyncTimeout(error_reset * 1000);
                }
            },
        ],
        afterResponse: [
            (_request, _options, response) => {
                if (response.status >= 400) {
                    error_limit--;
                }

                let esi_limit_headers = response.headers.get("x-esi-error-limit-remain");
                let esi_reset_headers = response.headers.get("x-esi-error-liimit-reset");
                if (esi_limit_headers && esi_reset_headers) {
                    error_limit = parseInt(esi_limit_headers);
                    error_reset = parseInt(esi_reset_headers);
                }

                return undefined;
            },
        ],
    },
});

export const esi_check = esi.extend({
    throwHttpErrors: false,
});

export default esi;

export async function all_batch<T>(promises: Promise<T>[]) {
    let results = [];
    for (let i = 0; i < promises.length; i += 5) {
        results.push(...(await Promise.all(promises.slice(i, i + 5))));
    }

    return results;
}