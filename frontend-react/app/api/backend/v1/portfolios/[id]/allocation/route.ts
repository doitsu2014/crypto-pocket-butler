import { NextRequest, NextResponse } from 'next/server';

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id } = await params;
  const backendUrl = process.env.BACKEND_URL || 'http://localhost:8080';
  
  try {
    const response = await fetch(`${backendUrl}/api/v1/portfolios/${id}/allocation`, {
      method: 'GET',
      headers: {
        'Authorization': request.headers.get('Authorization') || '',
      },
    });

    let data;
    try {
      data = await response.json();
    } catch (e: unknown) {
      console.error('Failed to parse portfolio allocation response as JSON:', e);
      return NextResponse.json(
        { error: 'Invalid response from backend' },
        { status: 502 }
      );
    }

    if (!response.ok) {
      return NextResponse.json(data, { status: response.status });
    }

    return NextResponse.json(data);
  } catch (error) {
    console.error('Error fetching portfolio allocation:', error);
    return NextResponse.json(
      { error: 'Failed to fetch portfolio allocation' },
      { status: 500 }
    );
  }
}
