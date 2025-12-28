mod common;

use common::TestDatabase;
use serial_test::serial;
use soar::clubs_repo::ClubsRepository;
use soar::users::User;
use soar::users_repo::UsersRepository;

async fn setup_test_db() -> TestDatabase {
    TestDatabase::new()
        .await
        .expect("Failed to create test database")
}

#[tokio::test]
#[serial]
async fn test_create_pilot_without_email() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool();
    let repo = UsersRepository::new(pool.clone());

    let pilot = User::new_pilot(
        "John".to_string(),
        "Doe".to_string(),
        true,  // is_licensed
        false, // is_instructor
        false, // is_tow_pilot
        false, // is_examiner
        None,  // No club association needed for this test
    );

    let result = repo.create_pilot(pilot.clone()).await;
    assert!(result.is_ok(), "Failed to create pilot: {:?}", result.err());

    // Verify pilot was created
    let created_pilot = repo.get_by_id(pilot.id).await.unwrap().unwrap();
    assert_eq!(created_pilot.first_name, "John");
    assert_eq!(created_pilot.last_name, "Doe");
    assert_eq!(created_pilot.email, None);
    assert_eq!(created_pilot.password_hash, None);
    assert!(created_pilot.is_licensed);
    assert!(!created_pilot.can_login());
    assert!(created_pilot.is_pilot());

    // Database automatically cleaned up when test_db goes out of scope
}

#[tokio::test]
#[serial]
async fn test_send_invitation_to_pilot() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool();
    let repo = UsersRepository::new(pool.clone());

    // Create pilot without email
    let pilot = User::new_pilot(
        "Jane".to_string(),
        "Smith".to_string(),
        true,
        false,
        false,
        false,
        None, // No club association needed for this test
    );
    repo.create_pilot(pilot.clone()).await.unwrap();

    // Add email and generate token
    let email = "jane.smith@example.com";
    let token_result = repo.set_email_and_generate_token(pilot.id, email).await;
    assert!(
        token_result.is_ok(),
        "Failed to set email and generate token: {:?}",
        token_result.err()
    );

    let token = token_result.unwrap();
    assert!(!token.is_empty(), "Token should not be empty");

    // Verify pilot now has email
    let updated_pilot = repo.get_by_id(pilot.id).await.unwrap().unwrap();
    assert_eq!(updated_pilot.email, Some(email.to_string()));
    assert_eq!(updated_pilot.password_hash, None);
    assert!(!updated_pilot.email_verified);
    assert!(!updated_pilot.can_login());

    // Verify we can retrieve pilot by token
    let pilot_by_token = repo.get_by_verification_token(&token).await.unwrap();
    assert!(pilot_by_token.is_some());
    assert_eq!(pilot_by_token.unwrap().id, pilot.id);

    // Database automatically cleaned up when test_db goes out of scope
}

#[tokio::test]
#[serial]
async fn test_complete_pilot_registration() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool();
    let repo = UsersRepository::new(pool.clone());

    // Create pilot and send invitation
    let pilot = User::new_pilot(
        "Bob".to_string(),
        "Johnson".to_string(),
        true,
        false,
        false,
        false,
        None, // No club association needed for this test
    );
    repo.create_pilot(pilot.clone()).await.unwrap();

    let email = "bob.johnson@example.com";
    let token = repo
        .set_email_and_generate_token(pilot.id, email)
        .await
        .unwrap();

    // Complete registration with password
    let password = "SecurePassword123!";
    let result = repo.set_password_and_verify_email(pilot.id, password).await;
    assert!(
        result.is_ok(),
        "Failed to complete registration: {:?}",
        result.err()
    );
    assert!(result.unwrap());

    // Verify pilot can now login
    let completed_pilot = repo.get_by_id(pilot.id).await.unwrap().unwrap();
    assert_eq!(completed_pilot.email, Some(email.to_string()));
    assert!(completed_pilot.password_hash.is_some());
    assert!(completed_pilot.email_verified);
    assert!(completed_pilot.can_login());

    // Verify password works
    let verify_result = repo.verify_password(email, password).await;
    assert!(verify_result.is_ok());
    assert!(verify_result.unwrap().is_some());

    // Verify token is no longer valid after registration
    let token_lookup = repo.get_by_verification_token(&token).await.unwrap();
    assert!(
        token_lookup.is_none(),
        "Token should be invalidated after registration"
    );

    // Database automatically cleaned up when test_db goes out of scope
}

#[tokio::test]
#[serial]
async fn test_full_pilot_invitation_workflow() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool();
    let repo = UsersRepository::new(pool.clone());

    // Step 1: Admin creates pilot without email
    let pilot = User::new_pilot(
        "Alice".to_string(),
        "Williams".to_string(),
        true,  // is_licensed
        true,  // is_instructor
        false, // is_tow_pilot
        true,  // is_examiner
        None,  // No club association needed for this test
    );
    repo.create_pilot(pilot.clone()).await.unwrap();

    // Verify pilot exists but cannot login
    let created_pilot = repo.get_by_id(pilot.id).await.unwrap().unwrap();
    assert!(!created_pilot.can_login());
    assert!(created_pilot.is_pilot());
    assert!(created_pilot.is_instructor);
    assert!(created_pilot.is_examiner);

    // Step 2: Admin sends invitation
    let email = "alice.williams@example.com";
    let _token = repo
        .set_email_and_generate_token(pilot.id, email)
        .await
        .unwrap();

    // Verify email was added but still cannot login
    let invited_pilot = repo.get_by_id(pilot.id).await.unwrap().unwrap();
    assert_eq!(invited_pilot.email, Some(email.to_string()));
    assert!(!invited_pilot.can_login());

    // Step 3: Pilot completes registration
    let password = "MySecurePass456!";
    repo.set_password_and_verify_email(pilot.id, password)
        .await
        .unwrap();

    // Verify pilot can now login
    let final_pilot = repo.get_by_id(pilot.id).await.unwrap().unwrap();
    assert!(final_pilot.can_login());
    assert!(final_pilot.email_verified);

    // Verify login works
    let login_result = repo.verify_password(email, password).await.unwrap();
    assert!(login_result.is_some());
    let logged_in_user = login_result.unwrap();
    assert_eq!(logged_in_user.id, pilot.id);

    // Database automatically cleaned up when test_db goes out of scope
}

#[tokio::test]
#[serial]
async fn test_get_pilots_by_club() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool();
    let users_repo = UsersRepository::new(pool.clone());
    let clubs_repo = ClubsRepository::new(pool.clone());

    // Create a test club first
    let club = clubs_repo
        .create_simple_club("Test Soaring Club")
        .await
        .unwrap();

    // Create multiple pilots for the club
    let pilot1 = User::new_pilot(
        "Pilot".to_string(),
        "One".to_string(),
        true,
        false,
        false,
        false,
        Some(club.id),
    );
    let pilot2 = User::new_pilot(
        "Pilot".to_string(),
        "Two".to_string(),
        false,
        true,
        false,
        false,
        Some(club.id),
    );
    let pilot3 = User::new_pilot(
        "Pilot".to_string(),
        "Three".to_string(),
        false,
        false,
        true,
        false,
        Some(club.id),
    );

    users_repo.create_pilot(pilot1.clone()).await.unwrap();
    users_repo.create_pilot(pilot2.clone()).await.unwrap();
    users_repo.create_pilot(pilot3.clone()).await.unwrap();

    // Get all pilots for the club
    let pilots = users_repo.get_pilots_by_club(club.id).await.unwrap();
    assert_eq!(pilots.len(), 3);

    // Verify all returned pilots are from the correct club
    for pilot in &pilots {
        assert_eq!(pilot.club_id, Some(club.id));
        assert!(pilot.is_pilot());
    }

    // Database automatically cleaned up when test_db goes out of scope
}

#[tokio::test]
#[serial]
async fn test_soft_delete_pilot() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool();
    let repo = UsersRepository::new(pool.clone());

    let pilot = User::new_pilot(
        "Delete".to_string(),
        "Me".to_string(),
        true,
        false,
        false,
        false,
        None, // No club association needed for this test
    );
    repo.create_pilot(pilot.clone()).await.unwrap();

    // Verify pilot exists
    let exists = repo.get_by_id(pilot.id).await.unwrap();
    assert!(exists.is_some());

    // Soft delete
    let deleted = repo.soft_delete_user(pilot.id).await.unwrap();
    assert!(deleted);

    // Verify pilot is no longer returned by get_by_id
    let after_delete = repo.get_by_id(pilot.id).await.unwrap();
    assert!(after_delete.is_none());
}

#[tokio::test]
#[serial]
async fn test_cannot_send_invitation_twice() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool();
    let repo = UsersRepository::new(pool.clone());

    let pilot = User::new_pilot(
        "Test".to_string(),
        "Pilot".to_string(),
        true,
        false,
        false,
        false,
        None, // No club association needed for this test
    );
    repo.create_pilot(pilot.clone()).await.unwrap();

    // Send first invitation
    let email = "test@example.com";
    let token1 = repo
        .set_email_and_generate_token(pilot.id, email)
        .await
        .unwrap();

    // Try to send second invitation - should generate new token
    let token2 = repo
        .set_email_and_generate_token(pilot.id, email)
        .await
        .unwrap();

    // Tokens should be different
    assert_ne!(token1, token2, "New invitation should generate new token");

    // Old token should be invalid
    let old_token_lookup = repo.get_by_verification_token(&token1).await.unwrap();
    assert!(
        old_token_lookup.is_none(),
        "Old token should be invalidated"
    );

    // New token should be valid
    let new_token_lookup = repo.get_by_verification_token(&token2).await.unwrap();
    assert!(new_token_lookup.is_some());

    // Database automatically cleaned up when test_db goes out of scope
}
