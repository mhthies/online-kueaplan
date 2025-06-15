from types import ModuleType


def test_list_categories(generated_api_client: ModuleType) -> None:
    EVENT_ID = 1
    client = generated_api_client.DefaultApi()
    client.api_client.configuration.host = "http://localhost:9099/api/v1"
    auth_response = client.authorize(EVENT_ID, generated_api_client.AuthorizeRequest(passphrase="user"))
    client.api_client.configuration.api_key["sessionTokenAuth"] = auth_response.session_token
    result = client.list_categories(EVENT_ID)
    assert len(result) == 1
    assert result[0].title == "Default"