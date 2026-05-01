# Swagger UI Guide

This guide explains how to use the Swagger UI to interact with the Crypto Pocket Butler API.

## Accessing Swagger UI

The Swagger UI is publicly accessible and does not require authentication to view the API documentation.

**URL:** `http://localhost:3000/swagger-ui`

When you access this URL, you'll see the interactive API documentation with all available endpoints.

## Authentication Methods

The API supports three authentication methods that can be used in Swagger UI:

### 1. Bearer Token (JWT)

This is the most common method for user authentication. You need to obtain a valid JWT token from Keycloak.

**Steps:**
1. Log in to your application web service or obtain a JWT token through Keycloak
2. In Swagger UI, click the **Authorize** button at the top
3. In the **bearer_auth** section, enter your token in the format: `Bearer <your-token>`
4. Click **Authorize**
5. Your subsequent API calls will include this token

**Example:**
```
Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
```

### 2. OAuth2 Client Credentials

This method is designed for service-to-service authentication where you have a client ID and client secret.

**Steps:**
1. In Swagger UI, click the **Authorize** button
2. In the **oauth2_client_credentials** section:
   - Enter your **Client ID** (from Keycloak client configuration)
   - Enter your **Client secret** (from Keycloak client "Credentials" tab)
3. Click **Authorize**
4. Swagger UI will automatically obtain an access token using the client credentials grant

**Prerequisites:**
- You need a Keycloak client configured with:
  - Client authentication: ON
  - Service accounts roles: ENABLED
  - Direct access grants: ENABLED (optional)

**To configure in Keycloak:**
1. Go to your Keycloak admin console
2. Select your realm
3. Go to **Clients** → Select/Create your client
4. In **Settings** tab:
   - Enable "Client authentication"
   - Enable "Service accounts roles"
5. In **Credentials** tab:
   - Copy the client secret

### 3. OAuth2 Authorization Code

This method is designed for user authentication through Keycloak's login page.

**Steps:**
1. In Swagger UI, click the **Authorize** button
2. In the **oauth2_authorization_code** section:
   - Enter your **Client ID**
   - Leave the client secret blank (public client) or enter it (confidential client)
3. Click **Authorize**
4. You will be redirected to Keycloak's login page
5. Enter your username and password
6. After successful login, you'll be redirected back to Swagger UI
7. Swagger UI will automatically obtain an access token

**Prerequisites:**
- You need a Keycloak client configured with:
  - Client authentication: ON (for confidential) or OFF (for public)
  - Standard flow: ENABLED
  - Valid redirect URIs must include: `http://localhost:3000/swagger-ui/oauth2-redirect.html`

**To configure in Keycloak:**
1. Go to your Keycloak admin console
2. Select your realm
3. Go to **Clients** → Select/Create your client
4. In **Settings** tab:
   - Enable "Standard flow"
   - Add redirect URI: `http://localhost:3000/swagger-ui/oauth2-redirect.html`
   - Set "Web origins" to: `http://localhost:3000`

## Keycloak Configuration

### Environment Variables

The OAuth2 URLs are automatically configured based on your environment variables:

```env
KEYCLOAK_SERVER=http://localhost:8080
KEYCLOAK_REALM=myrealm
KEYCLOAK_AUDIENCE=crypto-pocket-butler
```

### OAuth2 URLs

Based on your configuration, the following URLs are used:

- **Authorization URL:** `{KEYCLOAK_SERVER}/realms/{KEYCLOAK_REALM}/protocol/openid-connect/auth`
- **Token URL:** `{KEYCLOAK_SERVER}/realms/{KEYCLOAK_REALM}/protocol/openid-connect/token`

Example with default values:
- Authorization URL: `http://localhost:8080/realms/myrealm/protocol/openid-connect/auth`
- Token URL: `http://localhost:8080/realms/myrealm/protocol/openid-connect/token`

## Testing API Endpoints

Once authenticated, you can test any API endpoint:

1. Find the endpoint you want to test in the Swagger UI
2. Click on the endpoint to expand it
3. Click **Try it out**
4. Fill in the required parameters
5. Click **Execute**
6. View the response below

## Common Issues

### "401 Unauthorized" Error

**Possible causes:**
- Token has expired
- Invalid token format
- Token audience doesn't match `KEYCLOAK_AUDIENCE`
- Missing or invalid Authorization header

**Solutions:**
- Obtain a new token and re-authorize
- Verify your Keycloak configuration
- Check that the client ID in the token matches `KEYCLOAK_AUDIENCE`

### "CORS Error" in Browser Console

**Solution:**
- Add your Swagger UI URL to Keycloak client's "Web origins"
- In Keycloak client settings, set Web origins to: `http://localhost:3000`

### OAuth2 Redirect Not Working

**Solution:**
- Verify the redirect URI is configured in Keycloak:
  - `http://localhost:3000/swagger-ui/oauth2-redirect.html`
- Ensure the URL matches exactly (including protocol and port)

### Client Credentials Flow Not Working

**Solution:**
- Verify "Service accounts roles" is enabled in Keycloak client
- Check that client authentication is enabled
- Ensure you're using the correct client secret

## Security Notes

⚠️ **Important Security Considerations:**

1. **Never commit client secrets** to version control
2. **Use HTTPS in production** - HTTP is only acceptable for local development
3. **Rotate client secrets regularly** in production environments
4. **Limit client permissions** - Use role-based access control
5. **Monitor token usage** - Set appropriate token lifetimes in Keycloak
6. **Disable Swagger UI in production** or protect it with additional authentication

## Production Deployment

For production environments:

1. **Disable public Swagger UI access** or add additional authentication
2. **Use HTTPS** for all URLs
3. **Update KEYCLOAK_SERVER** to your production Keycloak URL
4. **Configure proper CORS settings**
5. **Use production-ready client configurations**
6. **Enable rate limiting** to prevent abuse

Example production configuration:
```env
KEYCLOAK_SERVER=https://auth.yourcompany.com
KEYCLOAK_REALM=production
KEYCLOAK_AUDIENCE=crypto-pocket-butler-prod
```

## Additional Resources

- [Keycloak Setup Guide](./KEYCLOAK_SETUP.md)
- [Web Setup Guide](./WEB_SETUP.md)
- [Keycloak OAuth2 Documentation](https://www.keycloak.org/docs/latest/securing_apps/)
