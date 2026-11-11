// External file: the dashboard CSP blocks inline scripts. The token is in the fragment, never in the query.
const token = location.hash.slice(1);
fetch('/api/version', { headers: { 'x-dashboard-token': token } });
