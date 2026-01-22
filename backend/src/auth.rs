/// JWT Authentication Module
/// 
/// Handles JWT token creation, validation, and user authentication.

use actix_web::{dev::ServiceRequest, Error, HttpMessage};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,        // Subject (user ID or identifier)
    pub exp: i64,           // Expiration time (Unix timestamp)
    pub iat: i64,           // Issued at (Unix timestamp)
    pub role: String,       // User role (admin, user, device, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>, // For device authentication
}

impl Claims {
    /// Create new claims for a user
    pub fn new(sub: String, role: String, device_id: Option<String>, expires_in_hours: i64) -> Self {
        let now = Utc::now();
        let exp = (now + Duration::hours(expires_in_hours)).timestamp();
        
        Self {
            sub,
            exp,
            iat: now.timestamp(),
            role,
            device_id,
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }
}

/// JWT token manager
pub struct JwtManager {
    secret: String,
}

impl JwtManager {
    /// Create new JWT manager with secret key
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    /// Generate JWT token
    pub fn generate_token(&self, claims: Claims) -> Result<String, String> {
        let encoding_key = EncodingKey::from_secret(self.secret.as_bytes());
        
        encode(&Header::default(), &claims, &encoding_key)
            .map_err(|e| format!("Failed to generate token: {}", e))
    }

    /// Validate and decode JWT token
    pub fn validate_token(&self, token: &str) -> Result<Claims, String> {
        let decoding_key = DecodingKey::from_secret(self.secret.as_bytes());
        let validation = Validation::default();

        decode::<Claims>(token, &decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|e| format!("Invalid token: {}", e))
    }

    /// Extract token from Bearer header
    pub fn extract_bearer_token(auth_header: &str) -> Option<String> {
        if auth_header.starts_with("Bearer ") {
            Some(auth_header[7..].to_string())
        } else {
            None
        }
    }
}

/// Middleware validator for JWT tokens
pub async fn jwt_validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "default_secret_change_in_production".to_string());
    
    let jwt_manager = JwtManager::new(secret);
    
    match jwt_manager.validate_token(credentials.token()) {
        Ok(claims) => {
            // Check if token is expired
            if claims.is_expired() {
                tracing::warn!("Expired token attempt for user: {}", claims.sub);
                return Err((
                    actix_web::error::ErrorUnauthorized("Token expired"),
                    req,
                ));
            }

            // Attach claims to request extensions for later use
            req.extensions_mut().insert(claims.clone());
            
            tracing::debug!("Authenticated request from user: {}, role: {}", claims.sub, claims.role);
            Ok(req)
        }
        Err(e) => {
            tracing::warn!("Invalid token: {}", e);
            Err((actix_web::error::ErrorUnauthorized("Invalid token"), req))
        }
    }
}

/// Helper to extract claims from request
pub fn get_claims_from_request(req: &actix_web::HttpRequest) -> Option<Claims> {
    req.extensions().get::<Claims>().cloned()
}

/// Check if user has required role
pub fn has_role(claims: &Claims, required_role: &str) -> bool {
    claims.role == required_role || claims.role == "admin"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_generation_and_validation() {
        let manager = JwtManager::new("test_secret".to_string());
        let claims = Claims::new(
            "test_user".to_string(),
            "user".to_string(),
            None,
            24,
        );

        let token = manager.generate_token(claims.clone()).unwrap();
        let validated_claims = manager.validate_token(&token).unwrap();

        assert_eq!(validated_claims.sub, claims.sub);
        assert_eq!(validated_claims.role, claims.role);
    }

    #[test]
    fn test_token_expiration_check() {
        let claims = Claims::new(
            "test_user".to_string(),
            "user".to_string(),
            None,
            24,
        );

        assert!(!claims.is_expired());
    }

    #[test]
    fn test_extract_bearer_token() {
        let header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let token = JwtManager::extract_bearer_token(header).unwrap();
        assert_eq!(token, "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    }

    #[test]
    fn test_invalid_bearer_format() {
        let header = "Basic dXNlcjpwYXNz";
        let token = JwtManager::extract_bearer_token(header);
        assert!(token.is_none());
    }

    #[test]
    fn test_role_checking() {
        let user_claims = Claims::new(
            "user1".to_string(),
            "user".to_string(),
            None,
            24,
        );

        let admin_claims = Claims::new(
            "admin1".to_string(),
            "admin".to_string(),
            None,
            24,
        );

        assert!(has_role(&user_claims, "user"));
        assert!(!has_role(&user_claims, "admin"));
        assert!(has_role(&admin_claims, "admin"));
        assert!(has_role(&admin_claims, "user")); // Admin can access user routes
    }
}
