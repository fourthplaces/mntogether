import { graphql } from "@/gql";
import "./fragments";

// ─── Queries ─────────────────────────────────────────────────────────────────

export const WidgetListQuery = graphql(`
  query WidgetList($widgetType: String, $countyId: ID, $search: String, $limit: Int, $offset: Int) {
    widgets(widgetType: $widgetType, countyId: $countyId, search: $search, limit: $limit, offset: $offset) {
      id
      widgetType
      authoringMode
      data
      zipCode
      city
      countyId
      county {
        id
        name
      }
      startDate
      endDate
      createdAt
      updatedAt
    }
  }
`);

export const WidgetDetailQuery = graphql(`
  query WidgetDetail($id: ID!) {
    widget(id: $id) {
      id
      widgetType
      authoringMode
      data
      zipCode
      city
      countyId
      county {
        id
        name
      }
      startDate
      endDate
      createdAt
      updatedAt
    }
  }
`);

export const EditionWidgetsQuery = graphql(`
  query EditionWidgets($editionId: ID!, $slottedFilter: String, $limit: Int, $offset: Int) {
    editionWidgets(editionId: $editionId, slottedFilter: $slottedFilter, limit: $limit, offset: $offset) {
      id
      widgetType
      authoringMode
      data
      zipCode
      city
      countyId
      county {
        id
        name
      }
      startDate
      endDate
      createdAt
      updatedAt
    }
  }
`);

// ─── Mutations ───────────────────────────────────────────────────────────────

export const CreateWidgetMutation = graphql(`
  mutation CreateWidget($widgetType: String!, $data: String!, $authoringMode: String, $zipCode: String, $city: String, $countyId: ID, $startDate: String, $endDate: String) {
    createWidget(widgetType: $widgetType, data: $data, authoringMode: $authoringMode, zipCode: $zipCode, city: $city, countyId: $countyId, startDate: $startDate, endDate: $endDate) {
      id
      widgetType
      authoringMode
      data
      zipCode
      city
      countyId
      county {
        id
        name
      }
      startDate
      endDate
      createdAt
      updatedAt
    }
  }
`);

export const UpdateWidgetMutation = graphql(`
  mutation UpdateWidget($id: ID!, $data: String, $zipCode: String, $city: String, $countyId: ID, $startDate: String, $endDate: String) {
    updateWidget(id: $id, data: $data, zipCode: $zipCode, city: $city, countyId: $countyId, startDate: $startDate, endDate: $endDate) {
      id
      widgetType
      authoringMode
      data
      zipCode
      city
      countyId
      county {
        id
        name
      }
      startDate
      endDate
      createdAt
      updatedAt
    }
  }
`);

export const UpdateWidgetDataMutation = graphql(`
  mutation UpdateWidgetData($id: ID!, $data: String!) {
    updateWidgetData(id: $id, data: $data) {
      id
      widgetType
      authoringMode
      data
      createdAt
      updatedAt
    }
  }
`);

export const DeleteWidgetMutation = graphql(`
  mutation DeleteWidget($id: ID!) {
    deleteWidget(id: $id)
  }
`);
