import os
from langchain_core.prompts import PromptTemplate
import json
from datetime import datetime
import openai
import asyncio

class OAICompatibleModelEndpoint:
    def __init__(self,
                 api_key,
                 base_url,
                 model,
                 temperature=0.0,
                 max_tokens=None,
                 system_prompt="default"
                 ):
        # use ENV variables
        self.model = model
        self.api_key = api_key
        self.temperature = temperature
        if base_url == "default":
            self.client = openai.OpenAI(
                api_key=self.api_key,
            )
        else:
            self.client = openai.OpenAI(
                base_url=base_url,
                api_key=self.api_key,
            )

        self.max_tokens = max_tokens
        self.date_context = datetime.now().strftime("%Y-%m-%d")

        if system_prompt == "default":
            system_prompt_search = PromptTemplate(input_variables=["date_today"], template="You are a helpful assistant that can answer questions and provide information.")
            self.system_prompt = system_prompt_search.format(date_today=self.date_context)
        else:
            self.system_prompt = system_prompt

    def chat_completion_async(self, query: str, contexts: list = []):
        if len(contexts) == 0:
            user_prompt = query
        else:
            user_prompt =  "User Question : " + query + "\n\n CONTEXTS :\n\n" + str(contexts)

        if self.model in ["o1-preview", "o1-mini"]:
            messages = [
                {"role": "user",
                 "content": f"System Instruction: {self.system_prompt} \n Instruction:{user_prompt}"}
            ]
        else:
            messages = [
                {"role": "system", "content": self.system_prompt},
                {"role": "user", "content": user_prompt}
            ]

        try:
            stream = self.client.chat.completions.create(
                model=self.model,
                messages=messages,
                stream=True,
                temperature=self.temperature,
                max_tokens=self.max_tokens
            )

            for chunk in stream:
                if chunk.choices[0].delta.content is not None:
                    yield chunk.choices[0].delta.content

        except Exception as e:
            print(f"Error during get_answer_fireworks call: {e}")
            yield "data:" + json.dumps(
                {'type': 'error',
                 'data': "We are currently experiencing some issues. Please try again later."}) + "\n\n"

    def chat_completion(self, query: str, contexts: list = []) -> str:
        all_chunks = []
        for chunk in self.chat_completion_async(query=query, contexts=contexts):
            all_chunks.append(chunk)
        full_response = "".join(all_chunks)
        return full_response

    #TODO: not tested yet.
    def get_relevant_questions(self, contexts, query):
        try:
            response = self.client.chat.completions.create(
                model=self.model,
                messages=[
                    {"role": "system", "content": "You are a helpful assistant that can answer questions and provide information."},
                    {"role": "user", "content": "User Query: " + query + "\n\n" + "Contexts: " + "\n" + contexts + "\n"}
                ],
                response_format={"type": "json_object"},
                temperature=self.temperature,
            )

            return response.choices[0].message.content
        except Exception as e:
            print(f"Error during RELEVANT FIREWORKS ***************: {e}")
            return {}

if __name__ == '__main__':
    from dotenv import load_dotenv
    load_dotenv()

    oai_llm = OAICompatibleModelEndpoint(
            api_key=os.getenv("MODEL_API_KEY"),
            base_url=os.getenv("SERVER_BASE_URL"),
            model=os.getenv("MODEL_NAME")
        )

    query = "What 2+2 answer in a very detailed way."
    contexts = []

    ############################################################################
    #  How to call the endpoint with streaming
    ############################################################################
    for chunk in oai_llm.chat_completion_async(query=query, contexts=contexts):
        print(chunk, end="")  # Print the streamed response as it arrives

    ############################################################################
    #  How to call the endpoint with streaming
    ############################################################################
    print('\n\n')
    print('#' * 200)
    result = oai_llm.chat_completion(query=query, contexts=contexts)
    print(result)



