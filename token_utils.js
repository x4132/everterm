const crypto = require('crypto');
const querystring = require('querystring');

let CLIENT_ID = process.env.CLIENT_ID;
let CLIENT_SECRET = process.env.CLIENT_SECRET;

console.log(CLIENT_ID, CLIENT_SECRET);

/**
 * Takes an authorization code and exchanges it for an access token and refresh token.
 *
 * @param {string} authorizationCode - The authorization code received from the SSO
 * @returns {Promise<Object>} A promise that resolves to an object containing the access token and refresh token
 */
async function requestToken(authorizationCode) {
    const basicAuth = Buffer.from(`${CLIENT_ID}:${CLIENT_SECRET}`).toString('base64');

    const headers = {
        'Authorization': `Basic ${basicAuth}`,
        'Content-Type': 'application/x-www-form-urlencoded'
    };

    const payload = querystring.stringify({
        grant_type: 'authorization_code',
        code: authorizationCode
    });

    try {
        const response = await fetch('https://login.eveonline.com/v2/oauth/token', {
            method: 'POST',
            headers: headers,
            body: payload
        });

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        return await response.json();
    } catch (error) {
        throw new Error(`Failed to request token: ${error.message}`);
    }
}

/**
 * Generates a URL to redirect the user to the SSO for authentication.
 *
 * @param {string[]} scopes - A list of scopes that the application is requesting access to
 * @param {string} redirectUri - The URL where the user will be redirected back to after the authorization flow is complete
 * @returns {[string, string]} A tuple containing the URL and the state parameter that was used
 */
function redirectToSso(scopes, redirectUri) {
    const state = crypto.randomBytes(8).toString('hex');

    const queryParams = {
        response_type: 'code',
        client_id: CLIENT_ID,
        redirect_uri: redirectUri,
        scope: scopes.join(' '),
        state: state
    };

    const queryString = querystring.stringify(queryParams);
    return [`https://login.eveonline.com/v2/oauth/authorize?${queryString}`, state];
}

module.exports = {
    requestToken,
    redirectToSso
};