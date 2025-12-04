import datetime
import uuid

from tests.conftest import ApiClientWrapper


def test_list_passphrases(generated_api_client: ApiClientWrapper) -> None:
    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "admin")
    passphrases = generated_api_client.client.list_passphrases(EVENT_ID)
    passphrases_by_id = {p.id: p for p in passphrases}

    user_passphrase = passphrases_by_id.get(1)
    assert user_passphrase is not None
    # passphrase itself should be obfuscated, but the last letter should be readable
    assert user_passphrase.passphrase.endswith("r")
    assert user_passphrase.passphrase != "user"
    assert user_passphrase.derivable_from_passphrase is None
    assert user_passphrase.role == "participant"

    user_derivable_passphrase = passphrases_by_id.get(4)
    assert user_derivable_passphrase is not None
    assert user_derivable_passphrase.passphrase is None
    assert user_derivable_passphrase.derivable_from_passphrase == 1
    assert user_derivable_passphrase.role == "participant-sharable"


def test_create_passphrase(generated_api_client: ApiClientWrapper) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "admin")
    passphrase = kueaplan_api_client.Passphrase(
        passphrase="verysecret",
        derivable_from_passphrase=None,
        role="orga",
    )
    generated_api_client.client.create_passphrase(EVENT_ID, passphrase)

    client2 = ApiClientWrapper(kueaplan_api_client)
    client2.login(EVENT_ID, "verysecret")

    authorization_info = client2.client.check_authorization(EVENT_ID)
    assert "orga" in [a.role for a in authorization_info.authorization]

    # Check that orga privileges are granted, by creating a new category
    category = kueaplan_api_client.Category(
        id=str(uuid.uuid4()),
        title="Test Category",
        icon="💡",
        color="ffaa00",
        sort_key=42,
    )
    client2.client.create_or_update_category(EVENT_ID, category.id, category)


def test_extended_attributes(generated_api_client: ApiClientWrapper) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "admin")
    passphrase = kueaplan_api_client.Passphrase(
        passphrase="the-secret-new-passphrase",
        derivable_from_passphrase=None,
        role="participant",
        comment="Secondary passphrase for testing purposes",
        valid_from=datetime.datetime(2025, 1, 1, 13, 37, 0, tzinfo=datetime.UTC),
        valid_until=datetime.datetime(2100, 1, 1, 0, 42, 0, tzinfo=datetime.UTC),
    )
    generated_api_client.client.create_passphrase(EVENT_ID, passphrase)

    passphrases = generated_api_client.client.list_passphrases(EVENT_ID)
    matching_passphrases = [
        passphrase
        for passphrase in passphrases
        if passphrase.passphrase is not None and passphrase.passphrase.endswith("rase")
    ]
    assert len(matching_passphrases) == 1, (
        f"New passphrase has not been found in list_passphrases output: {passphrases}"
    )

    returned_passphrase = matching_passphrases[0]
    assert returned_passphrase.valid_from == datetime.datetime(2025, 1, 1, 13, 37, 0, tzinfo=datetime.UTC)
    assert returned_passphrase.valid_until == datetime.datetime(2100, 1, 1, 0, 42, 0, tzinfo=datetime.UTC)
    assert returned_passphrase.comment == "Secondary passphrase for testing purposes"
